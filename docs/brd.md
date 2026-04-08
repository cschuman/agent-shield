# Agent Shield — Business Requirements Document

## Product Overview

Agent Shield is an AI Agent Audit Scanner that discovers, scores, and reports on AI agents in codebases. It consists of an open-source Rust CLI and a paid SaaS dashboard.

---

## Personas

### Primary: Sarah Chen — Staff Security Engineer
- Series C fintech, 400 employees
- SOC 2 auditor asked how they govern AI agents; she had no answer
- Budget: $50K-$100K discretionary
- Needs: Agent inventory, risk scores, audit-ready reports

### Secondary: Marcus Rivera — VP Compliance / GRC
- Mid-market pharma, 2,000 employees
- EU AI Act deadline pressure
- Budget: $100K-$300K for compliance tooling
- Needs: Regulatory framework mapping, custom compliance reports

### Tertiary: Dev (Developer adopter)
- Senior backend engineer, SaaS startup
- Saw agent-shield on HN, wants to scan their agents
- Budget: $0 (uses free CLI)
- Needs: Fast scan, terminal output, JSON export

---

## Feature Requirements

### P0 — Must Have for Launch

| Feature | Description | Tier |
|---|---|---|
| Agent Discovery | Detect agents across 10 frameworks via codebase scanning | CLI (Free) |
| Risk Scoring | 0-100 score per agent based on autonomy, tools, permissions, guardrails | CLI (Free) |
| Terminal Report | Colored, formatted risk report in terminal | CLI (Free) |
| JSON Export | Machine-readable scan results | CLI (Free) |
| NIST AI RMF Mapping | Map findings to NIST controls | CLI (Free) |
| User Authentication | Email/password + OAuth (GitHub, Google) via Better Auth | Dashboard (Team+) |
| Web Dashboard | Agent inventory grid, risk gauges, findings list | Dashboard (Team+) |
| Scan Upload | CLI uploads results to dashboard via API key | Dashboard (Team+) |
| PDF Report Generation | Board-ready risk scorecard as downloadable PDF | Dashboard (Team+) |
| Billing / Subscriptions | Stripe integration, per-agent-monitored metering | Dashboard (Team+) |
| Multi-tenancy | Organization-level data isolation via RLS | Dashboard (Team+) |
| Team Management | Invite members, role-based access (admin/member/viewer) | Dashboard (Team+) |

### P1 — Post-Launch (Month 2-3)

| Feature | Description | Tier |
|---|---|---|
| ISO 42001 Mapping | Map findings to ISO controls | Team+ |
| Historical Trends | Risk score over time, agent count over time | Team+ |
| Email Alerts | Notify on new agents, risk changes, threshold breaches | Team+ |
| CI/CD Integration | GitHub Actions, GitLab CI, Jenkins support | Business+ |
| Continuous Monitoring | Scheduled re-scans, drift detection | Business+ |
| EU AI Act Mapping | Map findings to EU AI Act articles | Business+ |
| OWASP Agentic Mapping | Map findings to OWASP Agentic Top 10 | Business+ |

### P2 — Growth (Month 4-6)

| Feature | Description | Tier |
|---|---|---|
| SSO / SAML | Enterprise identity provider integration | Enterprise |
| Custom Frameworks | User-defined compliance framework mapping | Enterprise |
| API Access | REST API for programmatic scan management | Enterprise |
| Insurance Data Export | Structured risk data for AI insurance underwriting | Enterprise |
| Audit Log | Immutable record of all user and system actions | Enterprise |
| On-Premise Scanner | Self-hosted CLI with air-gapped report upload | Enterprise |

### P3 — Future

| Feature | Description |
|---|---|
| Agent-level deep dive | Detailed per-agent page with tool inventory, permission map, system prompt analysis |
| Remediation automation | Auto-generate guardrail code snippets for detected gaps |
| Benchmark comparisons | "Your agents vs. industry average" anonymized benchmarks |
| Slack/Teams integration | Alert channels, scan triggers from chat |
| SOC 2 evidence export | Pre-formatted evidence packets for SOC 2 auditors |

---

## Legal / Compliance Triage

| Requirement | Applies? | Action |
|---|---|---|
| GDPR | Yes — if EU customers store scan results | Data processing agreement, EU data residency option |
| CCPA | Yes — if CA customers | Privacy policy, data deletion capability |
| SOC 2 Type II | Yes — customers will ask | Plan for certification by Month 9-12 |
| PCI-DSS | No — no payment card data stored (Stripe handles) | N/A |
| HIPAA | No — no health data | N/A |
| Accessibility (WCAG 2.1 AA) | Yes — dashboard must meet AA | Built into design system |

---

## Billing Requirements

### Metering
- Count unique agents per organization per billing period
- Agent = unique combination of (framework + file_path + agent_name) detected by scan
- Agent count determined by most recent scan, not cumulative
- Overage billing: pro-rated at tier-specific per-agent rate

### Billing States
```
trial → active → past_due → canceled
                          → reactivated
```

### Stripe Integration
- Products: Team, Business, Enterprise
- Metered billing: usage records submitted after each scan
- Checkout: Stripe Checkout for self-serve tiers
- Customer portal: Stripe Customer Portal for plan management
- Webhooks: subscription.created, updated, deleted, invoice.paid, invoice.payment_failed

### Free Tier
- No account required for CLI usage
- Dashboard requires account (free tier shows last 1 scan, 3 agents max)
- Upgrade prompt when exceeding free dashboard limits

---

## Non-Functional Requirements

| Requirement | Target |
|---|---|
| CLI scan time | <30s for 100K LOC codebase |
| Dashboard page load | <2s (p95) |
| API response time | <500ms (p95) |
| Uptime SLA | 99.5% (Team), 99.9% (Business/Enterprise) |
| Data retention | 12 months (Team), 24 months (Business), unlimited (Enterprise) |
| Concurrent scans | 10 per org (Team), 50 (Business), unlimited (Enterprise) |
