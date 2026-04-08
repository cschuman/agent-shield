# Agent Shield — Product Roadmap

## Phase Overview

```
Phase 1 (DONE)     Phase 2 (8 weeks)     Phase 3 (6 weeks)     Phase 4 (ongoing)
CLI Scanner         Dashboard MVP          Continuous             Enterprise +
                    + Billing              Monitoring             Insurance Data
───────────────────────────────────────────────────────────────────────────────
✅ Agent discovery  Auth + dashboard       Scheduled scans        SSO / SAML
✅ Risk scoring     Scan upload API        Drift detection        Custom frameworks
✅ Terminal report  PDF reports            CI/CD integration      Insurance data API
✅ JSON export      Stripe billing         Email alerts           On-premise option
✅ NIST mapping     Multi-tenancy          EU AI Act mapping      SOC 2 evidence
✅ 10 frameworks    Team management        OWASP mapping          Benchmarks
```

---

## Phase 1: CLI Scanner (COMPLETE)

**Status:** Shipped
**Duration:** 2 weeks
**Deliverables:**
- [x] Rust CLI binary (`cargo install agent-shield`)
- [x] Agent discovery across 10 frameworks
- [x] Risk scoring engine (0-100, NIST 4-tier autonomy)
- [x] Terminal report with colored output
- [x] JSON export
- [x] NIST AI RMF compliance mapping
- [x] Framework detection: LangChain, LangGraph, CrewAI, AutoGen, OpenAI Assistants, Anthropic MCP, Anthropic Agent SDK, AWS Bedrock, Vercel AI, Custom
- [x] Guardrail detection (input validation, output filtering, rate limiting, HITL, content filter, token limit, timeout, scope restriction)
- [x] Permission assessment (read, write, execute, admin)
- [x] Data access mapping (database, cloud storage, API, file system, email, webhook, system command)

---

## Phase 2: Dashboard MVP + Billing (8 weeks)

**Status:** Not started
**Goal:** First paying customer
**Team required:** Solo (Corey) — all within existing skillset

### Week 1-2: Foundation
- [ ] SvelteKit project setup (from existing coreyschuman.com patterns)
- [ ] Better Auth integration (email/password + GitHub OAuth)
- [ ] Neon Postgres database setup with Kysely
- [ ] Multi-tenant schema with RLS
- [ ] Organization model (create org, invite members)

### Week 3-4: Core Dashboard
- [ ] Scan upload API (CLI → dashboard via API key)
- [ ] Agent inventory view (table with risk scores, framework, permissions)
- [ ] Organization-level risk dashboard (overall score, distribution gauge)
- [ ] Findings list with severity badges and compliance references
- [ ] Scan history (list of past scans with timestamps)

### Week 5-6: Reports + Billing
- [ ] PDF report generation (board-ready risk scorecard)
- [ ] Stripe integration (Checkout, Customer Portal, webhooks)
- [ ] Usage metering (agent count per org per billing period)
- [ ] Tier enforcement (agent limits, feature gating)
- [ ] Upgrade/downgrade flow

### Week 7-8: Polish + Launch
- [ ] Onboarding flow (first scan walkthrough)
- [ ] Settings page (API keys, team management, billing)
- [ ] Landing page updates (link to dashboard signup)
- [ ] Marketing site deploy
- [ ] "Show HN" launch

### Key Decisions
- **Data model first:** Design all tables before writing application code
- **Billing is P0:** Product is not shippable without Stripe integration
- **RLS from day 1:** Multi-tenancy via Postgres RLS, not application-level filtering

---

## Phase 3: Continuous Monitoring (6 weeks)

**Status:** Not started
**Goal:** Business tier differentiation
**Prerequisite:** Phase 2 complete, 3+ paying customers

### Features
- [ ] Scheduled scans (cron-based, per-org configuration)
- [ ] Drift detection (alert when agent risk score changes significantly)
- [ ] CI/CD integration (GitHub Actions action, GitLab CI template)
- [ ] Email alerts (new agent detected, risk threshold breach, scan failure)
- [ ] ISO 42001 compliance mapping
- [ ] EU AI Act article mapping
- [ ] OWASP Agentic Top 10 mapping
- [ ] Historical trend charts (risk over time, agent count over time)
- [ ] Compliance readiness percentage per framework

### Hiring Trigger
If Phase 2 reaches 10+ paying customers, consider:
- Part-time contractor for frontend polish
- GRC domain expert partner (the "Geoff" for this space)

---

## Phase 4: Enterprise + Insurance Data (Ongoing)

**Status:** Not started
**Goal:** Enterprise tier and insurance data moat
**Prerequisite:** Phase 3 complete, $100K+ ARR

### Features
- [ ] SSO / SAML integration
- [ ] Custom compliance framework builder
- [ ] REST API for programmatic access
- [ ] Insurance data export API (structured risk data for underwriters)
- [ ] Audit log (immutable action history)
- [ ] On-premise scanner deployment option
- [ ] SOC 2 evidence export
- [ ] Anonymized benchmark comparisons
- [ ] Remediation code snippet generation

### Hiring Trigger
At $200K+ ARR:
- Full-time engineer #2
- Full-time compliance/GRC specialist (or partner)
- Part-time DevRel for conference talks + content

---

## Resource Reality

| Phase | Team | Duration | Capex |
|---|---|---|---|
| Phase 1 (CLI) | Solo | 2 weeks | $0 |
| Phase 2 (Dashboard) | Solo | 8 weeks | ~$200/mo (Neon + Fly.io) |
| Phase 3 (Monitoring) | Solo + maybe 1 contractor | 6 weeks | ~$500/mo |
| Phase 4 (Enterprise) | 2-3 people | Ongoing | Funded by revenue |

All phases are achievable solo. No hire is required until revenue justifies it.

---

## Kill Criteria

If these are not met, stop and reassess:

| Milestone | Deadline | Kill Criteria |
|---|---|---|
| CLI on GitHub | Done | N/A |
| First 3 design partners | Phase 2 Week 6 | Cannot find 3 people willing to try free dashboard |
| First paying customer | Phase 2 Week 8 | Zero revenue after 2 months of dashboard being live |
| 10 paying customers | Phase 2 + 3 months | Cannot reach $15K MRR within 5 months of dashboard launch |
| $100K ARR | Month 12 | Not on track → reassess product-market fit |
