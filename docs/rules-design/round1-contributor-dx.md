# Round 1: Contributor DX Design

**Expert**: DX Optimizer  
**Date**: 2026-04-24  
**Scope**: End-to-end contributor experience — from "I noticed a gap" to "shipped to users"

---

## Top 3 DX Principles for This Project

**1. The first PR must be completable in under 30 minutes by someone who has never written a rule.**

This is the single most important constraint. Not 2 hours. Not "a day if you read the docs." Thirty minutes. The rule format will not become an industry standard if it requires initiates. Semgrep's playground-to-PR path gets close to this; their mistake is that the in-browser editor still requires regex literacy. We need to go one layer deeper.

**2. The feedback loop must be entirely local before a contributor ever opens a PR.**

Nothing kills contribution velocity faster than discovering your rule is broken in CI. Semgrep's CI rejects rules for metadata quality, regex performance, and test coverage — but contributors only find out *after* pushing. Every check that CI runs must also be runnable with a single command locally, with output that is word-for-word identical. "It passed on my machine" must not be a possible sentence.

**3. DX is more important than schema elegance — and I will steel-man this hard.**

A beautiful schema that requires reading a 2000-word spec is worse than a slightly awkward schema with a working `agent-shield rule new` command that emits a starter file. The schema's job is to be learnable from examples, not from documentation. If the security-auditor team produces a schema that requires contributors to understand the difference between `detection.import` and `detection.code_pattern` without the tool explaining it, they have optimized for the wrong reader. Every schema field that a contributor gets wrong without a helpful error message is a schema design failure, not a contributor failure. We should be willing to add redundancy, aliases, and escape hatches to the schema — even if it makes the internal representation messier — if it reduces the cognitive load on first-time contributors.

---

## The First-PR Narrative: End-to-End Walkthrough

**Scenario**: Alex, a security researcher at Anthropic, notices that LangChain shipped `RunnableAgent` in v0.3.1 and `agent-shield scan` misses it. Alex has YAML literacy but no Rust background and has never contributed to this project.

---

### Step 1: Noticing the gap (0:00)

Alex runs `agent-shield scan ./my-langchain-project` and gets zero findings. The project clearly uses `RunnableAgent`. The CLI output should include, at the bottom of a zero-findings scan for a directory that contains Python files:

```
No agent frameworks detected.

Did we miss something? Run:
  agent-shield rule new --framework langchain --hint "RunnableAgent"

Or browse open detection gaps:
  https://github.com/org/agent-shield-rules/issues?label=detection-gap
```

This is the discoverability hook. Without it, Alex closes the terminal and moves on. The CLI must surface the path forward. This is non-negotiable.

**Time so far: 2 minutes.** Alex has read the output and knows what to do next.

---

### Step 2: Scaffolding the rule (2:00 - 7:00)

Alex runs:

```
agent-shield rule new --framework langchain --hint "RunnableAgent"
```

The command does four things:

1. Emits a `.yaml` starter file in `./agent-shield-rules/langchain-runnable-agent.yaml` with every required field pre-populated with placeholders and inline comments explaining each field.
2. Creates a `./agent-shield-rules/tests/langchain-runnable-agent/` directory with a `true-positive.py` and `true-negative.py` fixture stub.
3. Prints a 6-line "what to do next" summary to the terminal.
4. Opens the file in `$EDITOR` if set.

The starter file must be valid YAML that passes `agent-shield rule lint` immediately, even with placeholder values. Alex should never see a lint error caused by the scaffold. Only errors caused by changes Alex made.

**Time so far: 7 minutes.** Alex has a file open in their editor.

---

### Step 3: Writing the rule (7:00 - 18:00)

Alex edits the YAML. The only thing they need to know to write the detection pattern is basic Python import syntax — which they already know because they noticed the gap. The inline comment in the scaffold says:

```yaml
detection:
  # Patterns that identify this framework in source code.
  # Use import: for "from X import Y" style patterns.
  # Use code: for regex patterns against any line of code.
  # Tip: start with import: patterns — they have zero regex risk.
  imports:
    - "from langchain.agents import RunnableAgent"
    - "langchain.agents.RunnableAgent"
```

Alex adds the import patterns. They do not need to understand regex. Import detection is first-class because it covers 80% of cases and requires zero expertise.

For the risk fields, the scaffold pre-fills the framework's existing baseline with a comment: `# Inherits LangChain baseline (40). Adjust only if this API introduces meaningfully different risk.` Most contributors will leave this alone, which is correct behavior.

**Time so far: 18 minutes.**

---

### Step 4: Local testing (18:00 - 25:00)

Alex runs:

```
agent-shield rule test ./agent-shield-rules/langchain-runnable-agent.yaml
```

Output:

```
Testing: langchain-runnable-agent.yaml

  true-positive.py  ... PASS (1 match at line 3)
  true-negative.py  ... PASS (0 matches)

Lint:
  [PASS] Schema valid
  [PASS] Required fields present
  [PASS] Test fixtures: 1 true positive, 1 true negative
  [PASS] Regex safety: all patterns compiled in < 2ms
  [WARN] Rule has no compliance_mapping. Consider adding OWASP LLM06 or NIST AI RMF reference.
         Run: agent-shield rule map --rule langchain-runnable-agent.yaml --suggest

All checks passed (1 warning).
```

The warning is actionable. Alex runs the suggest command:

```
agent-shield rule map --rule langchain-runnable-agent.yaml --suggest
```

This emits a list of candidate compliance mappings with one-line descriptions and asks Alex to confirm or skip. This is how a GRC analyst who added a rule stub can participate without writing regex — the `map` command is the GRC path.

Also during `rule test`: if a regex pattern takes longer than 50ms on the fixture files, the output says:

```
  [FAIL] Regex performance: pattern "(?i)(?:some|catastrophic|pattern)..." took 340ms
         on true-positive.py (16KB). Patterns must complete in < 50ms per file.
         Tip: avoid variable-width lookahead. See: docs/rules-design/regex-safety.md
```

The contributor catches the catastrophic regex *locally*, before a maintainer ever sees it. The 50ms threshold is conservative relative to the 30-second scan target, which gives headroom for large codebases.

**Time so far: 25 minutes.**

---

### Step 5: Opening the PR (25:00 - 30:00)

Alex pushes to a fork and opens a PR against `agent-shield-rules`. The PR template auto-fills based on the rule file metadata. CI runs `agent-shield rule test` against every changed `.yaml` file. The CI output is byte-for-byte identical to what Alex saw locally — same command, same format, no surprises.

If CI fails, the failure message includes the exact local command to reproduce it:

```
FAIL: langchain-runnable-agent.yaml — true-negative.py matched unexpectedly at line 7.
Reproduce locally: agent-shield rule test ./langchain-runnable-agent.yaml --verbose
```

**Total time: 28 minutes.** PR is open.

---

## The Non-Coder Path (GRC Analyst)

A GRC analyst — knows OWASP Agentic Top 10, does not know regex — wants to add an LLM06 control mapping to an existing rule.

The path is:

1. Find the rule file in the repo (rules are named by framework, easily browsable).
2. Add or edit the `compliance_mapping` block in YAML. The schema for this block uses plain string keys (`owasp_agentic: LLM06`) not regex or code patterns.
3. Run `agent-shield rule lint` to confirm the mapping keys are valid.
4. Open a PR.

No regex required. No Rust required. CI validates that the control ID exists in the known control catalog and that the mapping is not circular. This is a 10-minute contribution. The `compliance_mapping` block is intentionally separated from the `detection` block in the schema so that these two concerns can be owned by different people.

We explicitly accept that the non-coder cannot write detection logic. That is a correct constraint. The productive friction for GRC contributors is that they must map to a real, enumerated control ID — we do not accept free-text control descriptions.

---

## CI Feedback Design

CI must do exactly three things, in this order, and stop at the first failure:

1. Schema validation with `agent-shield rule lint`. Errors are in the format: `line 14: 'risk_baseline' must be an integer between 0 and 100, got "high"`. No abstract JSON schema errors. Human-readable, always.

2. Test execution with `agent-shield rule test`. Each fixture is named, pass/fail is per-fixture. The contributor knows exactly which case failed and which line triggered (or failed to trigger).

3. Performance gate. Each pattern must compile and match against a 100KB synthetic fixture file in under 100ms. The synthetic fixture is checked into the repo so contributors can run this locally: `agent-shield rule bench ./my-rule.yaml`.

The maintainer should not need to diagnose regex catastrophe. CI catches it. The maintainer's job in review is purely human judgment:

- Does the description accurately describe what the rule detects?
- Does the risk baseline feel calibrated versus existing rules?
- Is the compliance mapping intellectually honest (not CYA-mapping everything to every control)?
- Does the rule have a clear true-negative case that would catch false positive regression?

Everything else is automated.

---

## The "Too Easy" Failure Mode and Productive Friction

If we make contribution trivial, we get: rules that detect import strings with no test fixtures, rules that claim CRITICAL risk for any LangChain usage, rules mapped to every compliance control as a hedge, and rules submitted by people who want their GitHub username in the contributors list but have not run a single test.

Productive friction lives in exactly three places:

**At submission:** CI requires at least one true positive and one true negative test fixture. No exceptions. This is Semgrep's strongest design decision. It takes 5 minutes and filters out almost all low-effort submissions. If your rule cannot produce a single true-negative example — a file that looks similar but should *not* match — you do not understand what your rule detects.

**At review:** Risk baseline changes require a comment justification in the PR. The PR template prompts: "If you changed risk_baseline, explain why in one sentence." A maintainer can reject a risk change with no review effort by simply asking "why?" The contributor must have done the thinking.

**At the schema level:** The `description` field has a minimum length of 80 characters enforced by the linter. Short descriptions are almost always insufficient. This is not about word count — it is a proxy for the contributor having actually thought through what they are detecting and why it matters. We are borrowing this idea from Falco's style guide, which requires `desc` to explain *why* the rule matters, not just *what* it matches.

The failure mode we are explicitly *not* introducing: requiring maintainer approval before CI runs, requiring a linked issue before a PR, requiring sign-off from a framework expert. These are all friction in the wrong place and they make the maintainer the bottleneck.

---

## Recognition and Attribution

Rules carry a `contributors` field in YAML:

```yaml
contributors:
  - github: alexsecurity
    added: "2026-04-24"
```

The `agent-shield scan` output, when a finding is triggered by a community rule, appends:

```
  Detected by rule: langchain-runnable-agent (contributed by @alexsecurity)
```

This is not optional or hidden in a CONTRIBUTORS file. It is in the tool output. When someone's rule catches a real agent in production, they see their name. This is the recognition loop that makes the ecosystem self-sustaining.

The CONTRIBUTORS file is auto-generated from the `contributors` fields across all rules as part of CI. The maintainer does not manage it manually.

---

## Maintainer Review Checklist

Automated (not on the checklist — CI handles it):
- Schema validity
- Test fixtures present and passing
- Regex performance
- Compliance mapping references valid control IDs
- CONTRIBUTORS auto-update

Human judgment only:
- [ ] Description is accurate and useful (does it explain what a non-expert should understand about this risk?)
- [ ] Risk baseline is calibrated relative to peer frameworks
- [ ] True-negative fixture actually tests a meaningful near-miss (not just an empty file)
- [ ] Compliance mapping is intellectually honest

A typical PR review should take under 10 minutes. If it takes longer, that is a signal that the contributor did not run local lint first — which means the local lint is broken and that is the real problem to fix.

---

## What Semgrep Gets Right (and Wrong)

Right: requiring true-positive and true-negative test files. This is the single highest-leverage quality gate and we should copy it verbatim.

Right: the playground-to-PR path. Lowering the barrier to writing the first rule without a local clone is powerful for casual contributors.

Wrong: the metadata burden for security-category rules is so heavy (CWE, OWASP, confidence, likelihood, impact, vulnerability class — all required) that it actively discourages contributions. The metadata belongs, but it should be optional on first submission and promptable by a `--suggest` command.

Wrong: CI error messages when the schema fails are generic JSON schema output. A contributor gets `required property 'metadata' not found at /rules/0` rather than `your rule is missing a 'description' field — add it under 'metadata'`. We will not repeat this.

What Falco gets right: the style guide requirement that `desc` explains *why* the rule matters forces contributors to articulate the threat model. We adopt this via the 80-character minimum.

What Falco gets wrong: the contribution bar requires familiarity with Falco's Sysdig filter expression language, which has no "import:" shortcut. The majority of agent detection cases are import-pattern detection, which requires no regex knowledge. We should make import detection first-class, not a special case of code pattern matching.

---

## Biggest Open Question I'm Punting to Round 2

**How do we handle rule quality that passes all automated checks but is genuinely wrong about risk?**

A rule can have a valid schema, passing test fixtures, fast regex, and a plausible compliance mapping, and still be dangerously miscalibrated — claiming CRITICAL risk for any LangChain import, or LOW risk for an agent with `os.system` calls. The automated system cannot catch this because it is a judgment call, not a format error.

The options are: (a) require a second maintainer review for any new `risk_baseline` value, (b) build a calibration CI step that flags outliers relative to the existing rule corpus, or (c) accept that miscalibrated rules will ship and rely on community correction via issues.

Option (b) is the most interesting — a statistical model of rule risk that alerts when a new rule's baseline is more than 2 standard deviations from the mean for its framework class — but it requires a rule corpus large enough to be meaningful. Until then, option (a) is the pragmatic answer. But this is a question for Round 2, where the schema team and the engine team will have a clearer picture of what "risk baseline" means mechanically.
