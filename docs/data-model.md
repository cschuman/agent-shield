# Agent Shield — Data Model

## Entity Relationship Diagram

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ organization │────<│    member     │>────│     user     │
└──────┬───────┘     └──────────────┘     └──────────────┘
       │
       │1:N
       │
┌──────┴───────┐     ┌──────────────┐
│   api_key    │     │ subscription │
└──────────────┘     └──────────────┘
       │                    │
       │                    │
┌──────┴───────┐            │
│    scan      │────────────┘
└──────┬───────┘
       │1:N
       │
┌──────┴───────┐
│    agent     │
└──────┬───────┘
       │1:N
       ├──────────────┐──────────────┐
┌──────┴───────┐┌─────┴──────┐┌──────┴───────┐
│   finding    ││    tool    ││  permission  │
└──────────────┘└────────────┘└──────────────┘
```

## DDL

```sql
-- ============================================================
-- EXTENSIONS
-- ============================================================
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================
-- USERS & ORGANIZATIONS
-- ============================================================

CREATE TABLE "user" (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ  -- soft delete
);

CREATE TABLE organization (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    tier TEXT NOT NULL DEFAULT 'free' CHECK (tier IN ('free', 'team', 'business', 'enterprise')),
    agent_limit INTEGER NOT NULL DEFAULT 3,  -- free tier limit
    stripe_customer_id TEXT UNIQUE,
    stripe_subscription_id TEXT UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE member (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES "user"(id),
    organization_id UUID NOT NULL REFERENCES organization(id),
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, organization_id)
);

-- ============================================================
-- API KEYS
-- ============================================================

CREATE TABLE api_key (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organization(id),
    created_by UUID NOT NULL REFERENCES "user"(id),
    name TEXT NOT NULL DEFAULT 'Default',
    key_hash TEXT NOT NULL UNIQUE,  -- bcrypt hash of the key
    key_prefix TEXT NOT NULL,       -- first 8 chars for display (e.g., "as_live_a1b2...")
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- SCANS
-- ============================================================

CREATE TABLE scan (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organization(id),
    triggered_by UUID REFERENCES "user"(id),  -- NULL for CLI-only scans
    source TEXT NOT NULL DEFAULT 'cli' CHECK (source IN ('cli', 'dashboard', 'ci', 'scheduled')),
    repository_url TEXT,
    repository_name TEXT,
    branch TEXT,
    commit_sha TEXT,
    scan_path TEXT NOT NULL,
    overall_risk_score SMALLINT NOT NULL CHECK (overall_risk_score BETWEEN 0 AND 100),
    risk_level TEXT NOT NULL CHECK (risk_level IN ('low', 'medium', 'high', 'critical')),
    agent_count INTEGER NOT NULL DEFAULT 0,
    finding_count INTEGER NOT NULL DEFAULT 0,
    compliance_framework TEXT NOT NULL DEFAULT 'nist',
    scan_duration_ms INTEGER,
    cli_version TEXT,
    status TEXT NOT NULL DEFAULT 'completed' CHECK (status IN ('running', 'completed', 'failed')),
    raw_json JSONB,  -- full CLI JSON output stored for reference
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- AGENTS (discovered in scans)
-- ============================================================

CREATE TABLE agent (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    scan_id UUID NOT NULL REFERENCES scan(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organization(id),
    name TEXT NOT NULL,
    framework TEXT NOT NULL,
    file_path TEXT NOT NULL,
    line_number INTEGER,
    risk_score SMALLINT NOT NULL CHECK (risk_score BETWEEN 0 AND 100),
    risk_level TEXT NOT NULL CHECK (risk_level IN ('low', 'medium', 'high', 'critical')),
    autonomy_tier SMALLINT NOT NULL CHECK (autonomy_tier BETWEEN 1 AND 4),
    tool_count INTEGER NOT NULL DEFAULT 0,
    guardrail_count INTEGER NOT NULL DEFAULT 0,
    has_system_prompt BOOLEAN NOT NULL DEFAULT FALSE,
    permission_summary TEXT,
    data_access_summary TEXT,
    system_prompt_preview TEXT,  -- first 500 chars
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- FINDINGS
-- ============================================================

CREATE TABLE finding (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent(id) ON DELETE CASCADE,
    scan_id UUID NOT NULL REFERENCES scan(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organization(id),
    category TEXT NOT NULL CHECK (category IN (
        'missing_guardrail', 'excessive_permission', 'data_exposure',
        'prompt_injection_risk', 'no_human_oversight', 'unbounded_autonomy',
        'missing_audit_trail'
    )),
    severity TEXT NOT NULL CHECK (severity IN ('info', 'low', 'medium', 'high', 'critical')),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    remediation TEXT NOT NULL,
    framework_ref TEXT NOT NULL,  -- e.g., "NIST AI RMF: GOVERN 1.7"
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- TOOLS (discovered on agents)
-- ============================================================

CREATE TABLE tool (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    has_confirmation BOOLEAN NOT NULL DEFAULT FALSE
);

-- ============================================================
-- PERMISSIONS (detected on agents)
-- ============================================================

CREATE TABLE permission (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id UUID NOT NULL REFERENCES agent(id) ON DELETE CASCADE,
    scope TEXT NOT NULL,
    level TEXT NOT NULL CHECK (level IN ('read', 'write', 'execute', 'admin'))
);

-- ============================================================
-- BILLING EVENTS (for metered billing)
-- ============================================================

CREATE TABLE billing_event (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organization(id),
    event_type TEXT NOT NULL CHECK (event_type IN (
        'subscription_created', 'subscription_updated', 'subscription_canceled',
        'invoice_paid', 'invoice_payment_failed', 'usage_reported',
        'tier_changed', 'trial_started', 'trial_ended'
    )),
    stripe_event_id TEXT UNIQUE,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- AUDIT LOG
-- ============================================================

CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    organization_id UUID NOT NULL REFERENCES organization(id),
    user_id UUID REFERENCES "user"(id),
    action TEXT NOT NULL,  -- e.g., 'scan.created', 'member.invited', 'api_key.revoked'
    resource_type TEXT,    -- e.g., 'scan', 'member', 'api_key'
    resource_id UUID,
    metadata JSONB,
    ip_address INET,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- INDEXES
-- ============================================================

CREATE INDEX idx_member_user ON member(user_id);
CREATE INDEX idx_member_org ON member(organization_id);
CREATE INDEX idx_api_key_org ON api_key(organization_id);
CREATE INDEX idx_api_key_hash ON api_key(key_hash);
CREATE INDEX idx_scan_org ON scan(organization_id);
CREATE INDEX idx_scan_created ON scan(organization_id, created_at DESC);
CREATE INDEX idx_agent_scan ON agent(scan_id);
CREATE INDEX idx_agent_org ON agent(organization_id);
CREATE INDEX idx_agent_risk ON agent(organization_id, risk_score DESC);
CREATE INDEX idx_finding_scan ON finding(scan_id);
CREATE INDEX idx_finding_org ON finding(organization_id);
CREATE INDEX idx_finding_severity ON finding(organization_id, severity);
CREATE INDEX idx_tool_agent ON tool(agent_id);
CREATE INDEX idx_permission_agent ON permission(agent_id);
CREATE INDEX idx_billing_org ON billing_event(organization_id);
CREATE INDEX idx_audit_org ON audit_log(organization_id, created_at DESC);

-- ============================================================
-- ROW LEVEL SECURITY
-- ============================================================

ALTER TABLE organization ENABLE ROW LEVEL SECURITY;
ALTER TABLE member ENABLE ROW LEVEL SECURITY;
ALTER TABLE api_key ENABLE ROW LEVEL SECURITY;
ALTER TABLE scan ENABLE ROW LEVEL SECURITY;
ALTER TABLE agent ENABLE ROW LEVEL SECURITY;
ALTER TABLE finding ENABLE ROW LEVEL SECURITY;
ALTER TABLE tool ENABLE ROW LEVEL SECURITY;
ALTER TABLE permission ENABLE ROW LEVEL SECURITY;
ALTER TABLE billing_event ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit_log ENABLE ROW LEVEL SECURITY;

-- RLS policies use a session variable: current_setting('app.organization_id')
-- Set via: SET LOCAL app.organization_id = '<uuid>';

CREATE POLICY org_isolation ON organization
    USING (id = current_setting('app.organization_id')::UUID);

CREATE POLICY member_isolation ON member
    USING (organization_id = current_setting('app.organization_id')::UUID);

CREATE POLICY api_key_isolation ON api_key
    USING (organization_id = current_setting('app.organization_id')::UUID);

CREATE POLICY scan_isolation ON scan
    USING (organization_id = current_setting('app.organization_id')::UUID);

CREATE POLICY agent_isolation ON agent
    USING (organization_id = current_setting('app.organization_id')::UUID);

CREATE POLICY finding_isolation ON finding
    USING (organization_id = current_setting('app.organization_id')::UUID);

CREATE POLICY tool_isolation ON tool
    USING (agent_id IN (
        SELECT id FROM agent WHERE organization_id = current_setting('app.organization_id')::UUID
    ));

CREATE POLICY permission_isolation ON permission
    USING (agent_id IN (
        SELECT id FROM agent WHERE organization_id = current_setting('app.organization_id')::UUID
    ));

CREATE POLICY billing_isolation ON billing_event
    USING (organization_id = current_setting('app.organization_id')::UUID);

CREATE POLICY audit_isolation ON audit_log
    USING (organization_id = current_setting('app.organization_id')::UUID);

-- ============================================================
-- SOFT DELETE STRATEGY
-- ============================================================
-- Position: soft delete on user and organization only.
-- All other entities use CASCADE from scan deletion.
-- Scan data is retained for billing/audit purposes even after org soft-delete.
-- Hard delete only via explicit admin action after retention period.

-- ============================================================
-- SEED DATA
-- ============================================================

-- Free user (for testing)
-- INSERT INTO "user" (email, name) VALUES ('dev@example.com', 'Dev User');
-- INSERT INTO organization (name, slug, tier, agent_limit) VALUES ('Dev Org', 'dev-org', 'free', 3);

-- Paid user (for testing)
-- INSERT INTO "user" (email, name) VALUES ('security@acme.com', 'Sarah Chen');
-- INSERT INTO organization (name, slug, tier, agent_limit) VALUES ('Acme Corp', 'acme', 'business', 100);

-- Admin user (for testing)
-- INSERT INTO "user" (email, name) VALUES ('admin@agentshield.dev', 'Admin');
```

## Migration Conventions

- File naming: `NNNN_description.sql` (e.g., `0001_initial_schema.sql`)
- All migrations are forward-only (no down migrations for MVP)
- Applied via Kysely migrator at app startup
- Review required before applying to production
- Schema changes tracked in git alongside application code

## Key Design Decisions

1. **organization_id on every tenant-scoped table** — enables RLS without JOINs
2. **scan.raw_json stores full CLI output** — denormalized for flexibility, structured tables for queries
3. **agent is per-scan, not global** — each scan creates new agent records; historical comparison done via scan timestamps
4. **tool and permission are child tables of agent** — CASCADE delete with scan cleanup
5. **billing_event is append-only** — immutable audit trail for all billing state changes
6. **audit_log is append-only** — immutable record of all actions for compliance
