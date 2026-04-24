# Round 2 — The Strategic Case Against Refactoring Rules to YAML Now

**Author:** Critic A (Strategy)
**Date:** 2026-04-24
**Charge:** Steel-man the position that the Round 1 refactor is wrong-now.

---

## 1. Verdict

**Refactor later, not now — and refactor narrower than proposed.** The Round 1 plans are technically excellent but premised on a contributor and customer audience that does not yet exist. The correct move is a one-week internal Rust seam (rust-pro's MVP only), then **stop**. Revisit YAML when one of three falsifiable triggers fires (listed in §5). Shipping a public YAML schema before product-market-fit creates lock-in on a guess.

---

## 2. The Case Against the Round 1 Plan

### Argument A — The opportunity cost is a paid product

The combined Round 1 scope is not "minimum viable refactor." Add the columns honestly:

- rust-pro Phase 0.2 (YAML loader, layered config, include_dir, schema versioning, quarantine diagnostics): ~2 weeks
- security-auditor schema v1.0 (8 primitives + combinators + `context_signal` + compliance block + worked examples + tag vocabulary): ~1 week of design plus ~1 week to implement loaders for each primitive
- backend-architect distribution (registry.yaml, minisign signing, `update-rules` subcommand, weekly CI release pipeline, `rules.agentshield.dev` hosting, schema migration shim): ~2 weeks
- dx-optimizer toolkit (`rule new`, `rule test`, `rule lint`, `rule bench`, `rule map --suggest`, fixture corpus, byte-identical CI parity, scaffold templates per primitive): ~2 weeks

Realistic total for a sole maintainer: **6–8 weeks of focused engineering**, not the "one-week refactor" the rust-pro doc opens with. That estimate covers only Phase 1 of the rust-pro plan; Phases 2 and 3 (AST matchers, signing verification, criterion benchmarks) push it past 12 weeks.

The agent-shield playbook (per project memory, completed 2026-04-08) flags **dashboard MVP and GRC partner conversations** as the revenue path. Six to eight weeks spent on rule infrastructure is six to eight weeks not spent on either. The dashboard is the thing someone pays for. Rules-as-data is the thing someone might fork.

**Concrete counter-example:** `cargo-audit` shipped its first three years with the advisory schema fully owned by RustSec maintainers and contributions arriving as PRs to a fixed YAML format defined ad-hoc in the `rustsec` repo — they did not build a contributor scaffolding CLI, signing pipeline, or schema versioning machinery until *after* the database mattered to industry. They got to relevance by curating data, not by building a contribution platform.

### Argument B — The standardization fantasy needs a customer

"Own the rule taxonomy" is a strong claim with weak current evidence. The reference projects all earned standardization tailwind *after* commercial validation:

- **Semgrep** open-sourced sgrep in 2018, but the rule registry only became a moat after r2c raised Series A in 2020 (~$5M ARR signals) — *two years and one funded company later*.
- **Trivy** was acquired by Aqua Security in 2019 before the misconfig/SBOM rule formats stabilized; the schema work happened with a corporate sponsor underwriting it.
- **Falco** went CNCF-incubated in 2018, three years after Sysdig built it as a commercial complement.

In all three cases, the rule format crystallized when there was a paying customer or foundation pulling for stability. Agent-shield has neither. A v1.0 schema designed in 2026-04 by one person, with no GRC partner ratifying the compliance mappings and no enterprise customer demanding override semantics, is a guess shipped as a contract. **Once the security-auditor's `extends:` field, `context_signal` enum, and `compliance:` per-framework keys ship publicly, breaking them is a multi-quarter migration tax forever.**

### Argument C — The bottleneck is awareness, not language

The dx-optimizer plan optimizes a 30-minute first-PR experience for a contributor who does not currently exist. Agent-shield is pre-launch, pre-GitHub-promotion, with zero outside contributors and no waitlist of frustrated would-be rule authors. The implicit thesis — that Rust is the contribution barrier — is not yet supported by data.

Counter-evidence: `cargo-deny`, `cargo-audit`, and `committed` accept Rust PRs from security-literate outsiders without YAML rule formats. The actual bottleneck for a sole-maintainer Rust security tool at 0 stars is **discovery**, not contribution mechanics. A rule format with no audience is infrastructure for a phantom.

The honest test: **how many issues currently exist on the agent-shield repo asking "how do I add a framework?"** If the answer is zero, building the contribution system is a solution looking for a problem. The first three contributors will learn Rust enough to copy a `DetectionPattern` literal. The fiftieth contributor is when YAML earns its keep — and the fiftieth contributor implies a level of project gravity that justifies the schema work *because the audience finally exists*.

---

## 3. What Round 1 Got Right

I am not strawmanning these. Each Round 1 expert produced something defensible that should survive Round 3 in some form.

- **rust-pro's MVP refactor (Phase 1 only)** is correct and should ship regardless. Compiling regex once, pre-bucketing by extension, adding `aho-corasick` prefilter, parallelizing with `rayon`, and extracting a `CompiledRule`/`Matcher` enum — these are pure performance and code-hygiene wins with zero schema commitment. Do this. It takes a week and produces no lock-in.
- **security-auditor's primitive analysis** is the right *content*. The eight primitives genuinely cover the existing `frameworks.rs` shapes. When YAML eventually lands, this primitive set is the answer. The work is not wasted; it is just not yet load-bearing.
- **backend-architect's bundled-primary distribution model** is correct and non-controversial. When YAML ships, `include_dir!` + offline-first + `update-rules` is the right pattern. Do not lazy-download; do not require network.
- **dx-optimizer's productive-friction principle** (true-positive + true-negative fixtures required, 80-char description minimum, recognition in scan output) is the right governance model when contributions exist. Keep it on the shelf.
- **Quarantine-on-bad-rule** (rust-pro's reliability invariant) is the single most important correctness property when YAML lands. Non-negotiable then; irrelevant now.

The Round 1 docs are not wrong about the destination. They are wrong about the timing.

---

## 4. The Path-of-Least-Regret Alternative

**"Internal Seam Now, Public Schema Later"** — sized at **5–7 working days** for a sole maintainer.

| Step | What | Time | Reversibility |
|---|---|---|---|
| 1 | Extract `agent-shield-rules` as an *internal* module (not a separate crate yet). Define `CompiledRule`, `Matcher` enum, `Engine`. Generate `CompiledRule` instances from existing `AgentFramework::detection_patterns()` via `From` impl. **No YAML loader. No public schema.** | 2 days | Trivially reversible — pure refactor, byte-identical output |
| 2 | Move regex compilation out of hot loops. Add `aho-corasick` literal prefilter. Parallelize per-file with `rayon`. | 1 day | Reversible — performance only |
| 3 | Unify `detect_guardrails` / `detect_data_access` / `detect_permissions` under the same `Matcher` enum so all detection is one mechanism (currently they are second-class, per rust-pro's correct observation). | 1 day | Reversible |
| 4 | Write a `CONTRIBUTING.md` showing how to add a framework by editing `frameworks.rs` — exactly the path Rust security folks already follow in `cargo-deny`, `cargo-audit`. **One concrete worked example, 200 lines of doc.** | 0.5 days | Trivially reversible |
| 5 | Add a single tracking issue: *"Considering YAML rule format — comment if you want to contribute detection rules."* Pin it. | 0.5 days | Free |

**What this buys:**
- All performance wins from rust-pro Phase 1
- A clean internal seam that makes the *future* YAML refactor a 2–3 week job instead of an 8-week job (because the engine boundary already exists)
- Zero public API surface to break later
- A measurable signal generator for the trigger conditions in §5
- 6+ weeks of maintainer time freed for dashboard MVP, GRC partner outreach, or detecting more frameworks (which is the actual product)

**What this defers:**
- Public YAML schema (security-auditor's full v1.0)
- Distribution pipeline (backend-architect's `update-rules` + minisign)
- Contributor scaffolding (dx-optimizer's `rule new` / `rule test`)

These are not abandoned. They are **deferred until the trigger fires**.

---

## 5. Signals That Would Change My Mind

Falsifiable, specific, time-boxed. If any one of these fires within the next ~6 months, refactor to full YAML on the Round 1 plan immediately.

1. **Contributor demand signal:** Five or more distinct GitHub users open issues, PRs, or comment on the pinned tracking issue asking how to add a framework or detection rule. Threshold: 5 people, 90-day window. *Rationale: this is the moment the Rust barrier becomes real friction on a real population.*

2. **Customer or partner pull signal:** Any one of the following: (a) a GRC partner conversation that reaches MoU/LOI stage where the partner wants to ship their own compliance mappings; (b) a paying or LOI customer requests custom rules or rule overrides as a contract requirement; (c) a foundation (CNCF, OpenSSF, Linux Foundation) expresses interest in adopting agent-shield's detection format. *Rationale: this is the moment "own the taxonomy" stops being aspirational.*

3. **Internal velocity signal:** The maintainer adds 3+ frameworks in a single sprint and finds the friction is in Rust ergonomics specifically (not in detection research, naming, scoring calibration, or compliance mapping). Measured by: time-to-add-framework exceeds 2 hours where the bottleneck is recompilation/Rust boilerplate, not detection logic. *Rationale: if the maintainer feels the pain firsthand on a representative workload, the refactor pays for itself in 6 months.*

4. **Competitive signal (defensive):** A direct competitor (Semgrep, Snyk, GitHub Advanced Security, Aikido, Endor Labs) ships agent-detection rules in their own format. *Rationale: at that point, the standardization race is on and defensive schema work becomes urgent.*

5. **Anti-trigger — explicitly do NOT refactor if:** in 90 days the project has <100 stars, <5 contributors, no commercial conversations, and the maintainer has not added a framework in the prior 30 days. That is a signal that the product needs distribution, not infrastructure.

---

## Closing

The Round 1 plans are good engineering pointed at the wrong quarter. Ship the internal seam. Talk to customers. Detect more frameworks. Watch the trigger conditions. When one fires, the security-auditor's primitive list, the backend-architect's distribution model, and the dx-optimizer's contribution UX are all sitting on the shelf ready to assemble — and by then, *they will be designed against a real audience instead of a hypothetical one*. That is when "own the taxonomy" stops being a phrase from a deck and starts being a defensible position.
