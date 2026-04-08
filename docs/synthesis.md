# Agent Shield — Research Synthesis

Single source of truth for all downstream documents. All strategic, operational, and technical docs reference this file.

---

## Validated Revenue Thesis

| Parameter | Value | Source |
|---|---|---|
| **Who pays** | CISOs, Security teams, Compliance/GRC teams (tiered) | Expert panel |
| **Pricing model** | Per-agent-monitored/month | Revenue Thesis Phase 0 |
| **Free tier** | Open-source Rust CLI, unlimited local scans | Competitive differentiation |
| **Team** | $1,500/mo (25 agents, $60/agent overage) | VC + CISO panel feedback |
| **Business** | $5,000/mo (100 agents, $50/agent overage) | CISO discretionary budget range |
| **Enterprise** | $15,000/mo (unlimited agents, custom) | Competitor pricing comparison |
| **Gross margin** | 95%+ (software, no claims exposure) | Unit economics analysis |
| **Year 1 ARR target** | $594K (15 Team + 5 Business + 1 Enterprise) | Back-of-napkin math |
| **Year 3 ARR target** | $9.3M (200 Team + 60 Business + 15 Enterprise) | Growth projection |

**Revenue thesis validated:** Competitors charge $100K+/yr with "contact sales." Our transparent, self-serve pricing at $18K-$180K/yr fills a gap confirmed by competitive scan. No direct competitor offers self-serve + transparent pricing + open-source CLI.

---

## Shared Metrics Table

| Metric | Year 1 | Year 2 | Year 3 |
|---|---|---|---|
| Team customers | 15 | 60 | 200 |
| Business customers | 5 | 20 | 60 |
| Enterprise customers | 1 | 5 | 15 |
| Total customers | 21 | 85 | 275 |
| ARR | $594K | $2.46M | $9.3M |
| CLI installs (free) | 5,000 | 25,000 | 100,000 |
| Agents monitored (paid) | 1,500 | 7,000 | 30,000 |
| MRR | $49.5K | $205K | $775K |
| Gross margin | 95% | 95% | 94% |
| Infra cost/agent/mo | ~$1.50 | ~$1.00 | ~$0.75 |

---

## Persona Cards

### Persona 1: Sarah Chen — Staff Security Engineer
- **Title:** Security Architect, Series C fintech (400 employees)
- **Emotional state:** Anxious. SOC 2 auditor asked "how do you govern AI agents?" and she had no answer. Developers are deploying agents without security review.
- **Budget:** $50K-$100K discretionary. Credit card for tools under $2K/mo.
- **Current solution:** Manual spreadsheet inventory. Updated quarterly (always stale).
- **Trigger:** SOC 2 audit finding or board question about AI risk.
- **Buys:** Team tier ($1,500/mo). Upgrades to Business when auditor asks for continuous monitoring.

### Persona 2: Marcus Rivera — VP of Compliance / GRC
- **Title:** VP Compliance, mid-market pharma (2,000 employees)
- **Emotional state:** Stressed. EU AI Act high-risk obligations hit Aug 2026. Legal is asking for AI governance documentation. He has none.
- **Budget:** $100K-$300K for compliance tooling. Procurement cycle 3-6 months.
- **Current solution:** Big 4 consulting engagement ($200K, point-in-time, 6-month lag).
- **Trigger:** EU AI Act deadline pressure or regulatory inquiry.
- **Buys:** Enterprise tier ($15,000/mo). Needs custom compliance framework mapping.

### Persona 3: Dev (Developer adopter)
- **Title:** Senior Backend Engineer, SaaS startup
- **Emotional state:** Curious but skeptical. Saw agent-shield on Hacker News. Wants to scan his LangChain agents.
- **Budget:** $0. Uses free CLI.
- **Current solution:** Nothing. Hopes for the best.
- **Trigger:** Blog post, HN post, colleague mention, or conference talk.
- **Buys:** Free CLI. Shares report with security team. Security team buys Team/Business.

---

## 20 Load-Bearing Data Points

### Market Size
1. AI insurance market projected $4.7B-$500B by 2030 (WTW / AIUC)
2. AI governance platform spending $492M in 2026, $2.54B total governance market (Gartner)
3. Global spending on AI governance/compliance projected $8.23B by 2034
4. AI-based research services market $8B (2025) → $35B (2035)
5. 38 US states adopted ~100 AI-related measures in 2025

### Competitive Landscape
6. Oasis Security raised $195M total, Fortune 500 only, no self-serve
7. Zenity raised $55M+, strong Microsoft ecosystem, enterprise-only
8. Lasso Security does agent discovery + audit trails, custom pricing
9. Safe Security AURA provides 0-100 AI Trust Score (closest concept competitor)
10. Snyk agent-scan is open-source but security-focused, not compliance-focused
11. Microsoft Agent Governance Toolkit is free + open-source (sets the floor)
12. Zero competitors offer transparent, public pricing
13. Robust Intelligence ($400M → Cisco), CalypsoAI ($180M → F5), Lakera (→ Check Point), Protect AI ($500-700M → Palo Alto) all acquired — talent consolidation

### Regulatory Drivers
14. EU AI Act high-risk obligations effective August 2026
15. Colorado SB 205 effective February 2026
16. NYC Local Law 144 requires bias audits costing $5K-$50K each, annually
17. ISO/IEC 42001 is certifiable with external audits (3-year validity)
18. OWASP published Agentic AI Top 10 for 2026 (first formal taxonomy)

### Technology
19. Only 38% of organizations monitor AI traffic end-to-end
20. 44% authenticate agents with static API keys (governance gap)
21. 79% operate with blindspots where agents invoke unobserved tools
22. 64% lack full visibility into AI risks
23. Agent memory infrastructure is contested (Mem0, Letta, Zep, Microsoft, Oracle)

### Insurance Layer
24. AI lawsuits grew 978% between 2021-2025
25. AIUC raised $15M seed (largest insurance seed ever), Nat Friedman + Ben Mann
26. Armilla AI offers $25M/org coverage for hallucinations, drift, data leakage
27. ISO CGL exclusion CG 40 47 covers 82% of US P&C policies — AI gaps widening
28. Traditional carriers (AXA XL, Hartford) adding exclusions, not coverage

---

## Document Theses

| Document | Central Argument |
|---|---|
| **Vision** | Agent Shield fills the critical gap between AI agent deployment and AI agent governance by being the first self-serve, transparent-pricing audit platform — the "Vanta for AI agents." |
| **Competitive Positioning** | Every funded competitor is enterprise-only and sales-led. Agent Shield wins by owning the SMB/mid-market wedge with open-source distribution and transparent pricing before incumbents move downmarket. |
| **GTM Plan** | Distribution comes from the open-source CLI (developer adoption → internal champion → security team purchase), not from outbound sales. First 10 customers come from the SOC 2 compliance crowd. |
| **BRD** | The MVP is the CLI (shipped) + web dashboard + PDF reports. Payment integration is P0. Multi-tenancy and team features gate the Team tier. |
| **Roadmap** | Phase 1 (CLI) is done. Phase 2 is dashboard + auth + billing. Phase 3 is continuous monitoring. Phase 4 is compliance framework expansion + insurance data API. |
| **Tech Architecture** | Rust CLI + SvelteKit dashboard + Go API + Neon Postgres. CLI scans locally and optionally uploads results to dashboard. Dashboard is the monetization layer. |
