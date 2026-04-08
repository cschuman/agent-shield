# Agent Shield — Technical Architecture

## C4 Level 2: Container Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                          USERS                                      │
│                                                                     │
│  Developer (CLI)         Security Team (Dashboard)    CI/CD System  │
└────────┬──────────────────────┬──────────────────────────┬──────────┘
         │                      │                          │
         │ cargo install        │ HTTPS                    │ API
         │                      │                          │
┌────────▼──────────┐  ┌───────▼──────────┐      ┌───────▼──────────┐
│   Rust CLI        │  │  SvelteKit App   │      │  GitHub Action   │
│                   │  │  (Dashboard)      │      │  (CI Integration)│
│  - Scanner engine │  │  - SSR pages      │      │                  │
│  - Risk scoring   │  │  - API routes     │      │  Calls REST API  │
│  - Report gen     │  │  - Auth (Better)  │      │  with API key    │
│  - JSON export    │  │  - Stripe billing │      └──────────────────┘
│  - Upload to API  │  │  - PDF generation │
└────────┬──────────┘  └───────┬──────────┘
         │                      │
         │ POST /api/scans      │ SQL (Kysely)
         │ (API key auth)       │
         │                      │
         └──────────┬───────────┘
                    │
            ┌───────▼──────────┐     ┌──────────────────┐
            │  Neon Postgres   │     │  Tigris S3       │
            │                  │     │                  │
            │  - All tables    │     │  - PDF reports   │
            │  - RLS enabled   │     │  - Scan archives │
            │  - pg-boss jobs  │     │                  │
            └──────────────────┘     └──────────────────┘
                    │
            ┌───────▼──────────┐
            │  Stripe          │
            │                  │
            │  - Subscriptions │
            │  - Metered usage │
            │  - Webhooks      │
            └──────────────────┘
```

## API Surface

### Public API (API key auth)

| Method | Path | Auth | Description |
|---|---|---|---|
| POST | /api/v1/scans | API key | Upload scan results from CLI |
| GET | /api/v1/scans | API key | List scans for organization |
| GET | /api/v1/scans/:id | API key | Get scan detail with agents and findings |
| GET | /api/v1/agents | API key | List all agents across scans |
| GET | /api/v1/agents/:id | API key | Get agent detail with tools, permissions, findings |
| GET | /api/v1/reports/:scanId/pdf | API key | Download PDF report for a scan |
| GET | /api/v1/compliance/:framework | API key | Get compliance readiness for a framework |

### Dashboard API (session auth via Better Auth)

| Method | Path | Auth | Description |
|---|---|---|---|
| GET | /api/dashboard/overview | Session | Dashboard stats (agent count, risk score, findings) |
| GET | /api/dashboard/agents | Session | Agent inventory with filters/sort |
| GET | /api/dashboard/findings | Session | Findings list with severity filter |
| GET | /api/dashboard/compliance | Session | Compliance readiness across frameworks |
| GET | /api/dashboard/trends | Session | Historical risk score and agent count trends |
| POST | /api/dashboard/api-keys | Session | Generate new API key |
| DELETE | /api/dashboard/api-keys/:id | Session | Revoke API key |
| GET | /api/dashboard/team | Session | List team members |
| POST | /api/dashboard/team/invite | Session | Invite team member |
| DELETE | /api/dashboard/team/:id | Session | Remove team member |

### Webhook Endpoints

| Method | Path | Auth | Description |
|---|---|---|---|
| POST | /api/webhooks/stripe | Stripe signature | Handle Stripe events |

## Auth Model

```
Better Auth (library-based)
├── Email/Password signup
├── GitHub OAuth
├── Session management (httpOnly cookies)
├── Organization context (set on login, stored in session)
└── API key auth (separate path for CLI/CI)
    ├── Key format: as_live_<32-random-chars>
    ├── Stored as bcrypt hash in api_key table
    ├── Sent via Authorization: Bearer <key> header
    └── Resolved to organization_id for RLS
```

## Security Model

| Concern | Approach |
|---|---|
| **CORS** | Dashboard origin only (app.agentshield.dev). API endpoints accept any origin with valid API key. |
| **CSP** | Strict CSP with nonce-based script loading. No inline scripts in dashboard. |
| **Rate Limiting** | 100 req/min per API key (scan uploads). 1000 req/min per session (dashboard). Implemented via pg-boss or in-memory counter. |
| **Input Validation** | Zod schemas on all API inputs. Scan upload limited to 10MB JSON. |
| **Secrets** | All secrets in environment variables. No secrets in code or config files. |
| **SQL Injection** | Kysely parameterized queries. No raw SQL with user input. |
| **XSS** | SvelteKit auto-escapes. CSP blocks inline scripts. |
| **CSRF** | SvelteKit built-in CSRF protection for form submissions. |
| **Data Isolation** | Postgres RLS policies on every tenant-scoped table. organization_id set via session variable per request. |

## CLI ↔ Dashboard Integration

```
1. User signs up on dashboard
2. User generates API key (as_live_abc123...)
3. User configures CLI:
   $ agent-shield config set api-key as_live_abc123...
   $ agent-shield config set endpoint https://app.agentshield.dev

4. User runs scan with upload:
   $ agent-shield scan . --upload

5. CLI scans locally (all processing on client machine)
6. CLI POSTs JSON results to /api/v1/scans
7. Dashboard displays results

Key principle: ALL scanning happens locally on the CLI.
The dashboard only receives results — it never accesses customer code.
This is critical for security trust: "your code never leaves your machine."
```

## Background Jobs (pg-boss)

| Job | Schedule | Description |
|---|---|---|
| `usage.report` | Hourly | Report agent count to Stripe for metered billing |
| `scan.cleanup` | Daily | Delete raw_json from scans older than retention period |
| `report.generate` | On demand | Generate PDF report (async, notify when ready) |
| `alert.check` | After each scan | Check alert rules, send email notifications |
| `subscription.sync` | Daily | Sync Stripe subscription status with local tier |
