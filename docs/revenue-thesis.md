# Revenue Thesis: Agent Shield

## Who Pays

Three buyer personas, tiered by sophistication and budget:

| Tier | Buyer | Title | Budget Authority | Pain |
|---|---|---|---|---|
| **Starter** ($500/mo) | Engineering / DevOps | Staff Engineer, DevOps Lead | Self-serve, credit card | "I deployed 12 agents and have no idea what permissions they have" |
| **Pro** ($2,500/mo) | Security / CISO office | CISO, Security Architect | $50-150K discretionary | "My SOC 2 auditor asked how we govern AI agents and I had no answer" |
| **Enterprise** ($8,000/mo) | Compliance / GRC | VP Compliance, Chief Risk Officer | $100K+ with procurement | "EU AI Act hits Aug 2026 and we need documented AI governance" |

## Pricing Model

**Per-agent-monitored/month** with tier-based feature gating.

| | Starter | Pro | Enterprise |
|---|---|---|---|
| **Price** | $500/mo | $2,500/mo | $8,000/mo |
| **Agents included** | Up to 25 | Up to 100 | Unlimited |
| **Overage** | $20/agent/mo | $25/agent/mo | Custom |
| **CLI scanner** | Unlimited | Unlimited | Unlimited |
| **Dashboard** | Basic | Full | Full + SSO |
| **Risk scorecard PDF** | Monthly | Weekly | Real-time |
| **Compliance frameworks** | NIST AI RMF | + ISO 42001 | + EU AI Act, state laws |
| **Continuous monitoring** | No | Yes | Yes |
| **Auditor-ready reports** | No | Yes | Yes + custom |
| **Support** | Community | Email | Dedicated CSM |

## Distribution Strategy

**Open-source Rust CLI** (free, forever) is the top of funnel. Think: `npm audit` adoption path.
- Developer installs CLI, scans their agents, gets a terminal risk report
- Shares report internally → security team sees it → asks "can we get this for all our agents?"
- Security team signs up for Pro dashboard
- Compliance team adds Enterprise for regulatory framework mapping

## What the Current Alternative Costs

| Current Solution | Cost | Problem |
|---|---|---|
| Manual agent inventory (spreadsheets) | 40-80 hr/quarter of engineer time (~$10K-$20K) | Always stale, no risk scoring |
| Big 4 AI audit engagement | $150K-$500K per engagement | Point-in-time, 3-6 month lag |
| Credo AI / Holistic AI governance platforms | $50K-$200K/yr | Policy management, not agent-level scanning |
| Nothing (most companies) | $0 upfront, unlimited liability | "We'll deal with it when something breaks" |

## What Would Make Someone Cancel After Month 1

- Scanner doesn't detect their agent framework (only supports LangChain but they use custom agents)
- Risk scores feel arbitrary / not actionable
- PDF report isn't polished enough for board presentation
- No integration with their CI/CD pipeline
- Dashboard UX feels like a developer tool, not a security tool

## Back-of-Napkin Revenue Math

**Conservative (Year 1):**
- 20 Starter customers: 20 x $500 x 12 = $120K
- 8 Pro customers: 8 x $2,500 x 12 = $240K
- 2 Enterprise customers: 2 x $8,000 x 12 = $192K
- **Year 1 ARR: $552K**

**Moderate (Year 2):**
- 80 Starter, 30 Pro, 8 Enterprise
- **Year 2 ARR: $1.73M**

**Aggressive (Year 3, with regulatory tailwinds):**
- 300 Starter, 100 Pro, 25 Enterprise
- **Year 3 ARR: $6.6M**

## Gross Margin

- Infrastructure cost per agent monitored: ~$0.50-$2/mo (compute for scanning, storage for results)
- At $20-$80/agent/mo pricing: **95%+ gross margin**
- This is a software business, not an insurance business — no claims exposure

## The Moat

1. **Data compounding**: Every scan generates structured risk data. Over time, this becomes the actuarial dataset that AI insurers need to price policies. Nobody else has this.
2. **Framework coverage**: Supporting 10+ agent frameworks (LangChain, CrewAI, AutoGen, OpenAI Assistants, Anthropic MCP, custom) creates switching costs.
3. **Compliance templates**: As regulations proliferate (38 states, EU, sector-specific), the compliance mapping library becomes a defensible asset.
4. **Open-source community**: The free CLI creates network effects and contributor momentum.
