# Agent Shield — UX Journey Map

## Journey Overview

```
AWARENESS → DISCOVERY → FIRST SCAN → INTERNAL SHARE → TEAM ADOPTION → PAID CONVERSION → RETENTION
   |             |           |              |               |               |              |
  Blog/HN    GitHub      CLI install    Share report    Security team    Stripe        Continuous
  Talk       README      + scan        with CISO       evaluates        checkout      monitoring
```

---

## Stage 1: Awareness

**Trigger:** Developer sees a blog post, HN thread, conference talk, or colleague mention about AI agent security risks.

**Emotional state:** Curious but skeptical. "Is this real, or is it fear-mongering?"

**Touchpoints:**
- Blog post: "I scanned our AI agents and found 4 critical risks"
- Hacker News "Show HN: agent-shield — npm audit for AI agents"
- Conference lightning talk at BSides/DevSecOps
- Colleague shares scan report in Slack

**Screen:** Landing page (index.html)

**Key moment:** The terminal mockup showing a real scan output. This is the "oh, I need to try this" moment.

**Success metric:** Click-through to GitHub README

---

## Stage 2: Discovery

**Trigger:** Developer clicks through to GitHub, reads README.

**Emotional state:** Evaluating. "Is this worth 5 minutes of my time?"

**Touchpoints:**
- GitHub README with demo GIF
- Feature list and framework support
- MIT license confirmation
- Star count / community signals

**Screen:** GitHub README (not in product)

**Key moment:** Seeing their framework listed (LangChain, CrewAI, etc.) in the supported list.

**Success metric:** `cargo install agent-shield`

---

## Stage 3: First Scan

**Trigger:** Developer runs `agent-shield scan .` in their project directory.

**Emotional state:** Anticipation → surprise. "I didn't know we had 7 agents... and one has admin access?!"

**Touchpoints:**
- CLI installation (cargo install, <30 seconds)
- First scan (<30 seconds for typical codebase)
- Terminal report with risk scores
- Findings with specific remediation

**Screen:** Terminal output (CLI)

**Key moment:** The risk score and critical findings. This is the "holy shit" moment that drives internal sharing.

**Critical UX requirements:**
- Scan must complete in under 30 seconds
- Report must be visually striking (colors, table formatting)
- Findings must be actionable (specific fix, not generic advice)
- Zero configuration required (just point at a directory)

**Failure mode:** Scan finds nothing (false negative) → user thinks tool doesn't work → abandons. Must have broad enough detection patterns.

**Success metric:** User runs a second scan on another project OR shares the report.

---

## Stage 4: Internal Share

**Trigger:** Developer shows the scan report to their security lead or engineering manager.

**Emotional state:** Helpful but cautious. "I found something useful but I don't want to create a fire drill."

**Touchpoints:**
- Screenshot of terminal report in Slack
- JSON export attached to a Jira ticket
- PDF report sent to CISO (if available)
- Verbal: "Hey, have you seen this tool?"

**Screen:** Terminal report OR JSON export OR PDF

**Key moment:** Security lead's reaction. If they say "can we run this on all our repos?" → success.

**Critical UX requirements:**
- JSON export must be clean and parseable
- Report must look professional enough that a developer isn't embarrassed to share it
- Risk scores must feel credible (not everything is CRITICAL)

**Success metric:** Security lead requests team-wide deployment

---

## Stage 5: Team Adoption

**Trigger:** Security team decides to evaluate Agent Shield for org-wide use.

**Emotional state:** Analytical. "Does this meet our requirements? Is it worth paying for?"

**Touchpoints:**
- Pricing page (pricing.html)
- Dashboard preview (dashboard.html)
- Comparison with alternatives (competitive positioning)
- Trial signup

**Screens:** Pricing page, Dashboard preview, Trial onboarding

**Key moment:** Seeing the dashboard preview with compliance readiness bars. "This is what our board needs to see."

**Critical UX requirements:**
- Pricing must be transparent (no "contact sales" for Team/Business)
- Dashboard must look like an enterprise tool, not a developer toy
- Trial must be frictionless (signup → first scan upload → dashboard in <5 minutes)
- Compliance framework mapping must be specific (article numbers, not vague references)

**Failure mode:** Dashboard looks too simple → security team doesn't trust it for enterprise use. Must balance developer simplicity with enterprise credibility.

**Success metric:** Trial signup

---

## Stage 6: Paid Conversion

**Trigger:** Trial period ends OR security team decides to purchase.

**Emotional state:** Calculating ROI. "Is $1,500/mo worth it vs. manual spreadsheet?"

**Touchpoints:**
- Stripe Checkout
- Plan selection
- Team invite flow
- First paid scan

**Screens:** Checkout, Billing management, Team invite

**Key moment:** The ROI conversation. $1,500/mo vs. $10K-$20K/quarter in manual audit time.

**Critical UX requirements:**
- Checkout in <60 seconds (Stripe Checkout, not custom form)
- Immediate access after payment (no waiting for "account provisioning")
- Clear understanding of what they're paying for (agent count, features)
- Easy to add team members

**Failure mode:** Procurement blocks credit card purchase → need invoice billing for Business/Enterprise.

**Success metric:** First payment processed

---

## Stage 7: Retention

**Trigger:** Ongoing use of dashboard, continuous monitoring.

**Emotional state:** Routine → occasional urgency. "I check this weekly... wait, why did the risk score spike?"

**Touchpoints:**
- Weekly dashboard check
- Email alerts (new agent, risk change)
- PDF report for monthly board presentation
- Compliance readiness tracking
- Re-scan after code changes

**Screens:** Dashboard (primary), Reports, Alerts, Settings

**Key moment:** The risk spike alert. When continuous monitoring detects a new agent with high risk, the alert email justifies the subscription.

**Critical UX requirements:**
- Dashboard must load fast (<2s)
- Email alerts must be actionable (link directly to the finding)
- PDF reports must be consistently formatted and professional
- Historical trends show value over time ("your risk score improved 15 points this quarter")

**Failure mode:** Nothing changes → tool feels unnecessary → churn. Must provide ongoing value through trend tracking and new regulation mapping.

**Success metric:** Month 2 retention, Month 6 retention

---

## Screen-to-Stage Mapping

| Stage | Screen | Type | Priority |
|---|---|---|---|
| Awareness | Landing page (index.html) | High-fi | ✅ Done |
| Discovery | GitHub README | Markdown | TODO |
| First Scan | CLI terminal output | Terminal | ✅ Done |
| Internal Share | JSON/PDF export | CLI output | Partial (JSON done, PDF TODO) |
| Team Adoption | Pricing page (pricing.html) | High-fi | ✅ Done |
| Team Adoption | Dashboard preview (dashboard.html) | High-fi | ✅ Done |
| Paid Conversion | Signup / Login | Low-fi | TODO |
| Paid Conversion | Checkout flow | Low-fi | TODO |
| Paid Conversion | Onboarding | Low-fi | TODO |
| Retention | Dashboard (live) | Build in Phase 2 | Phase 2 |
| Retention | Settings / API keys | Low-fi | TODO |
| Retention | Agent detail page | Low-fi | TODO |

---

## Emotional Arc

```
Curious → Skeptical → Surprised → Concerned → Relieved → Confident → Routine
  |          |            |           |            |           |          |
 "What's   "Does it    "We have    "Some have   "Now we    "Board     "Check
  this?"   actually    HOW many     admin        know        report    weekly,
           work?"      agents?!"   access?!"    what to     is        flag
                                                 fix"       ready"    spikes"
```
