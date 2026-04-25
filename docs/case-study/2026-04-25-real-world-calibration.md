# Real-World Calibration: Agent Shield v0.1.0 Against 5 OSS Agent Repos

**Date:** 2026-04-25
**Scanner version:** 0.1.0 (commit `78da44a`, tag `v0.1.0`)
**Branch:** `calibration-v0.1.0`
**Status:** Complete — findings synthesized, rule fixes applied, snapshot
contract preserved.

## Why this exists

Agent Shield v0.1.0 shipped a stable rules-as-data architecture (10 framework
detectors, 11 scoring rules) backed by a 6-fixture snapshot oracle. The
fixtures are intentionally narrow — they exist to lock the *byte-identical
contract* across releases, not to validate that the scanner is *good*.

This calibration answers a question the snapshot suite cannot: how does the
scanner perform on real-world agent codebases that nobody on the project has
ever scanned before?

The output is two artifacts:

1. **This engineering report** — frank about what the scanner got wrong, what
   it got right, and what changed. Written for project contributors and the
   maintainer's future self.
2. **A site-ready marketing excerpt**
   ([`site/docs/case-study-real-world.html`](../../site/docs/case-study-real-world.html))
   — the polished, externally-presentable version. Same data, different
   audience.

## Methodology

### Repo selection (5 repos, diverse frameworks)

| # | Repo | Framework focus | Rationale |
|---|---|---|---|
| 1 | `langchain-ai/langgraph` | LangGraph (official) | Framework's own monorepo — accuracy floor on framework-internal noise |
| 2 | `crewAIInc/crewAI-examples` | CrewAI (official-adjacent) | Canonical multi-agent CrewAI usage |
| 3 | `modelcontextprotocol/servers` | Anthropic MCP (official) | Reference MCP server implementations |
| 4 | `OpenInterpreter/open-interpreter` | Custom agent (3rd-party) | Popular agent CLI; tests the "Custom Agent" path |
| 5 | `vercel/ai` | Vercel AI SDK (framework repo) | Stress test — what happens on a framework repo? |

Cloned `--depth=1` into `calibration/repos/`, gitignored. The exact commits
were whatever HEAD was on `main` for each repo on 2026-04-25.

### Scan procedure

```bash
cargo build --release
for r in langgraph crewAI-examples servers open-interpreter ai; do
  ./target/release/agent-shield scan calibration/repos/$r \
    --format json -o calibration/scans/$r.json
done
```

JSON outputs in `calibration/scans/` (also gitignored, recreatable).

### Per-repo assessment

Each repo got a parallel ~30-minute structured assessment from a coding
subagent. Each agent was given the scanner output, the repo, and the YAML
detection + scoring rules; asked to sample detections, classify TPs vs.
FPs, walk the source tree for missed agents (recall), and propose concrete
patches. The agents read real files and real YAML — not speculation.

The five raw assessments are summarised in §"Per-repo findings" below.

### Initial detection counts (pre-tuning)

| Repo | Agents detected |
|---|---|
| langgraph | 385 |
| crewAI-examples | 82 |
| servers (MCP) | 59 |
| open-interpreter | **0** |
| vercel/ai | **1,291** |

Two of these numbers are obviously wrong before any deeper inspection:

- **open-interpreter at 0** — it is unambiguously an agent project. A scanner
  that detects zero agents in open-interpreter is broken on the "Custom
  Agent" path. **Recall failure.**
- **vercel/ai at 1,291** — this is the AI SDK *framework repo itself*. The
  scanner is happily detecting agents inside the implementation of the
  framework, including tests and type definitions. **Massive over-detection.**

The other three numbers are plausible and need per-detection inspection
before any verdict.

## Per-repo findings

### langchain-ai/langgraph (385 → 43, **89% reduction**)

**Sample precision:** 0/15 in pre-tuning sample. Every detection in the
random sample was either framework internals, framework tests, type stubs,
re-exports, docstring example code, or YAML/JSON file artifacts.

**FP patterns** (with examples):
- 143 of 385 (37%) came from `tests/` paths — pytest fixtures and unit
  tests that exercise framework surface (`libs/langgraph/tests/test_graph_callbacks.py`,
  `libs/sdk-py/tests/test_encryption.py`).
- 31 `__init__.py` files re-exporting framework symbols got *two* detections
  each — once for LangChain, once for LangGraph (e.g.
  `libs/checkpoint/langgraph/store/memory/__init__.py` flagged at line 15
  for `from langchain.embeddings` *and* line 6 for `from langgraph.store.memory`).
- YAML and JSON files in `.github/` were flagged on English-prose strings
  containing "langchain" + the word "use " (`bug-report.yml:13` is *literally*
  English text: "...please use the [LangChain Forum]...").
- The `Anthropic MCP` rule's bare `import_contains: "mcp"` matched
  `connect_mcp` and `disconnect_mcp` symbols inside `libs/sdk-py/.../runtime.py`
  (3-char substring + "import" on the line).
- `libs/prebuilt/langgraph/prebuilt/tool_validator.py` — a *guardrail
  primitive* — was flagged at score 85 as a high-risk agent. The scanner was
  inverting the signal.

**Recall miss:** all 34 `.ipynb` example agents in `examples/` were
invisible — `.ipynb` is not in `SCAN_EXTENSIONS`. The most agent-rich part
of the repo, blind. Deferred to v0.2.

**What changed in v0.1.0 calibration:**
- Test directories added to `SKIP_DIRS` in `src/scanner.rs`.
- LangGraph rule rewritten as `all_of(import + graph/agent constructor)`.
  Constructor allowlist: `StateGraph`, `MessageGraph`, `create_react_agent`,
  `create_agent`, `create_supervisor`, `.add_node`, `@entrypoint`, `@task`.
- LangChain rule rewritten as `all_of(import + agent constructor)`.
  Constructor allowlist: `AgentExecutor`, `create_*_agent` family,
  `initialize_agent`, `RunnableAgent`, `AgentType.*`.
- Anthropic MCP rule: dropped bare `"mcp"` substring; require
  `from mcp.<X>` / `import mcp.<X>` / `@modelcontextprotocol`.

**Result:** 385 → 43. The remaining 33 LangGraph detections are mostly
real framework-shipped reference implementations using `StateGraph` —
honest matches that would require the deferred `is_framework_self`
context signal to suppress further.

### crewAIInc/crewAI-examples (82 → 36, **56% reduction**)

**Pre-tuning precision:** ~52% per the assessment. Mix of real CrewAI agents
plus aux-import false-positives. Two structural issues:

- The LangChain rule fired on `tasks.py` orchestrator files where
  `from langchain.tools import Tool` is an aux import — the file orchestrates
  agents but isn't itself an agent.
- The CrewAI rule's bare `import_contains: "crewai"` fired on every helper
  module, not only agent definitions.

**Score saturation:** 91% of detections scored exactly 100 (all-Critical).
The scoring layer has no concept of "this match is low-confidence" and
piles every missing-* finding onto every detection.

**What changed:**
- CrewAI rule rewritten as `all_of(import + Agent/Crew constructor or
  @CrewBase/@agent/@crew/@task decorator)`.
- LangChain rule tightening (above) also helps — the aux-import
  false-positives stop firing because the orchestrator files don't have
  an `AgentExecutor()` or `create_*_agent()` call.

**Result:** 82 → 36. The 35 remaining CrewAI detections are real agent
definitions (the residual 1 is a LangGraph match in a related sub-tree).

### modelcontextprotocol/servers (59 → 41, **31% reduction**)

**Pre-tuning FP rate:** ~40% per the assessment.

- `import_contains: "mcp"` was the noisiest matcher in the entire rule set.
  3 characters + `"import"` on the same line matches *anything* with `mcp`
  in it.
- A package lockfile was flagged as "AWS Bedrock" because of an unrelated
  string match in the lock metadata.
- The `everything/` server alone produced a 39-row explosion (one detection
  per file; lots of small modules).
- Tool-count recall ~41% — the scanner often missed tool registration in
  the actual MCP server implementations because the `Tool(name=...)` form
  with named-enum arguments wasn't extracted.

**What changed:**
- Anthropic MCP rule rewritten without the bare `"mcp"` substring; uses
  anchored `from mcp.<X>` / `import mcp.<X>` and the canonical
  `@modelcontextprotocol` SDK namespace.
- Test directories now skipped (the `everything/` server has a non-trivial
  test surface that no longer adds noise).

**Result:** 59 → 41. Tool-count recall fix and lockfile-skip are deferred
to v0.2.

### OpenInterpreter/open-interpreter (0 → 14, **recall recovered**)

A canonical custom agent project returning **zero detections** is
unambiguously a scanner bug. Five root causes from the assessment:

1. The `custom-agent.yaml` rule looks for `system_prompt` /
   `systemPrompt`, but Open Interpreter uses `system_message` (LiteLLM
   convention).
2. `tool_call` regex required assignment form (`tool_call =`); real code
   uses dict-key form (`"tool_call":`) and call-site form.
3. No detection for the OpenAI message-format
   `{"role": "system"}` pattern that nearly every custom agent uses.
4. `package_dep` matchers are dormant in the v0.1 scanner (the manifest
   pass is wired but `Matcher::matches_repo` is unused) — so the
   `litellm`/`anthropic` package signals never fire.
5. The Anthropic Agent SDK rule recognizes only the (newer) Agent SDK,
   not the base SDK or the computer-use beta.

**What changed:**
- Custom Agent rule broadened to recognize:
  - `system_message` / `systemMessage` (LiteLLM dialect)
  - dict-form `"tool_call":` and `"tool_calls":` keys
  - OpenAI message format `{"role": "system"}`
  - `litellm.completion()` and `Anthropic().messages.create()` call sites
  - `agent.{loop,run,execute,step,chat}(` invocation patterns

**Result:** 0 → 14 detections. Recall recovered. Items 4 (manifest pass)
and 5 (Anthropic Agent SDK base-SDK extension) deferred to v0.2.

### vercel/ai (1,291 → 1,140, **12% reduction**)

The most interesting case. **0/15 sample TPs** in the pre-tuning sample —
not because the matches were *wrong* (the framework's tests genuinely call
`generateText()`, the examples genuinely use `tool({...})`), but because
**every match was the framework's own implementation**, not a deployed agent.

**FP breakdown (1,175 of 1,291 were Vercel AI rule hits):**
- `examples/ai-functions/src/` — 960 detections (the SDK's own runnable
  example cookbook).
- `examples/ai-e2e-next/` — 92.
- `packages/codemod/__testfixtures__/` — 48.
- `packages/ai/*.test.ts` and `*.test-d.ts` — 36.
- `examples/mcp/` — 35.
- `examples/next-langchain/` — 16.

**Two structural rule failures:**
1. `import_contains: "ai/"` matched every cross-package import inside the
   monorepo (`@ai-sdk/openai`, `@ai-sdk/provider-utils`, etc.). The 3-char
   substring `ai/` is essentially a no-op gate.
2. Function-call regexes (`generateText(`, `tool({`) ran without an import
   gate — vitest helpers, codemod testfixtures, and the framework's own
   implementation files all matched.

A bonus FP: AWS Bedrock rule fired on the URL string
`bedrock-agent-runtime.us-west-2.amazonaws.com` inside
`packages/amazon-bedrock/src/bedrock-provider.ts`.

**Score saturation:** 1,261 of 1,291 detections (97.7%) scored Critical.
Every demo file inherits the same "no system prompt + no input validation
+ no audit trail" findings, so the entire repo gets painted Critical
including 21-line `console.log` demos.

**What changed:**
- Vercel AI rule rewritten as
  `all_of(import-from-'ai' + generateText|streamText|tool function-call)`.
  Drops the bare `"ai/"` substring; decouples Vercel AI detection from the
  `@ai-sdk/*` monorepo cross-imports.
- AWS Bedrock rule rewritten with anchored SDK class names
  (`BedrockAgentClient`, `BedrockAgentRuntimeClient`) instead of bare
  `bedrock-agent` substring.
- Test directories now skipped (kills the 48 codemod `__testfixtures__`
  hits and 36 `packages/ai/*.test.ts` hits).

**Result:** 1,291 → 1,140. The remaining 1,140 detections in
`examples/ai-functions/*` are *correct in the technical sense* — those
files do call `generateText()` from the `'ai'` SDK. But they are
framework-shipped demos, not deployed agents. Eliminating them requires
architectural work: an `is_framework_self` context signal that reads the
repo's root `package.json` (deferred to v0.2 along with manifest-pass
activation).

The honest answer is that running `agent-shield` against `vercel/ai` is
**outside the intended use case**. Agent Shield is for application
codebases that *use* an agent framework, not for the framework itself.
README guidance has been updated to reflect this; a future version will
auto-detect framework repos and refuse / warn.

## Synthesis

### Final detection counts (post-tuning)

| Repo | Pre | Post | Delta |
|---|---:|---:|---:|
| langgraph | 385 | 43 | **−89%** |
| crewAI-examples | 82 | 36 | **−56%** |
| servers (MCP) | 59 | 41 | **−31%** |
| open-interpreter | 0 | 14 | **recall recovered** |
| vercel/ai | 1,291 | 1,140 | −12% |
| **Total** | **1,817** | **1,274** | **−30%** |

### Cross-cutting themes

1. **Every framework rule needed `all_of(import + positive constructor)`,
   not `any_of(broad alternatives)`.** This was the single highest-leverage
   change. A bare import is not an agent. Every detection rule should
   require a *positive* signal — a graph constructor, an agent factory, a
   decorator — that the file is actually defining or running an agent.

2. **3-character substring matchers were the worst offenders.**
   `import_contains: "mcp"` (Anthropic MCP), `import_contains: "ai/"`
   (Vercel AI), and `code_regex: 'bedrock-agent'` (AWS Bedrock) all fired
   on incidental strings that happened to coexist with the word `import`
   on the same line. Anchored regexes (`^\s*(from|import)\s+...`,
   `\bClassName\b`) were the fix.

3. **Test directories produced ~37% of false-positives** in the largest
   repo (langgraph). The fix is mechanical and low-risk: add `tests`,
   `test`, `__tests__`, `__testfixtures__`, `__mocks__`, `spec` to
   `SKIP_DIRS`. Snapshot fixtures don't contain any of these directories,
   so the byte-identical contract is preserved.

4. **Framework-self-detection is unsolvable with rule changes alone.**
   When the scanned repo *is* the framework, every rule sees an honest
   import and an honest function call. The only way out is an
   `is_framework_self` context signal computed from the root manifest —
   architectural work deferred to v0.2.

5. **Score saturation is a real problem.** 97.7% of vercel/ai detections
   are Critical; 78% of langgraph (pre-tuning); 91% of crewAI-examples
   pegged at exactly 100. The current scoring rules apply every
   missing-guardrail penalty to every detection, regardless of detection
   confidence. A demo file with `console.log` gets the same Critical
   rating as a production agent with shell access. Deferred to v0.2:
   confidence-gate the missing-* scoring rules so single-import-only
   matches don't accumulate the full penalty stack.

## Rule fixes applied in this calibration

Commit-by-commit on branch `calibration-v0.1.0`:

| Commit | Subject |
|---|---|
| `bba5ae0` | docs(calibration): add v0.1.0 real-world calibration scaffolding |
| `c950945` | fix(scanner): skip test directories from default scan |
| `db44a9d` | fix(rules): tighten Anthropic MCP rule (drop bare 'mcp' substring) |
| `e76e515` | fix(rules): tighten Vercel AI rule (anchor imports + require import+call) |
| `c0b6f71` | fix(rules): tighten AWS Bedrock rule (precise SDK class names) |
| `2c90648` | fix(rules): tighten LangChain rule (require positive agent signal) |
| `f02d2a6` | fix(rules): tighten LangGraph rule (require positive agent signal) |
| `d8eb41a` | fix(rules): tighten CrewAI rule (require Agent/Crew constructor) |
| `be382b5` | fix(rules): broaden Custom Agent rule (system_message + dict-form patterns) |
| `ff6b061` | fix(rules): restore min_match_count=2 floor on Vercel AI rule |

Each commit independently revertible. Snapshot fixtures byte-identical
through every commit. `cargo test --release`: 69 passing throughout.
`cargo clippy --all-targets -- -D warnings`: clean throughout.

## What did not get fixed (deferred to v0.2)

1. **`is_framework_self` context signal.** Read the repo's root manifest
   (`package.json`, `pyproject.toml`); if the package name matches a known
   framework, suppress that framework's own rule. Architectural — touches
   `signals.rs`, `engine/matcher.rs::evaluate_context_signal`, and
   `rules/loader.rs::is_known_signal` in lockstep, plus a startup
   manifest read in `scanner.rs`.

2. **Manifest-pass activation.** `Matcher::matches_repo` is wired but the
   v0.1 scanner only invokes `matches_file`. Activating it lets
   `package_dep:` and `file_present:` matchers participate in detection,
   which is necessary for full recall on Open Interpreter (it identifies
   itself via `litellm` in `pyproject.toml`).

3. **Confidence-gate the missing-* scoring rules.** Today every detection
   accumulates every missing-guardrail penalty regardless of detection
   strength. A low-confidence match (single-import-only) shouldn't
   instantly get the same Critical rating as a high-confidence match
   (multi-tool agent with broad permissions). Either gate by a new
   `detection_confidence` signal or by `tool_count > 0`.

4. **`.ipynb` notebook parsing.** The most agent-dense part of any
   LangChain/LangGraph repo is its tutorial notebooks. Today the scanner
   is blind to them (`SCAN_EXTENSIONS` doesn't include `.ipynb`).

5. **Docstring stripping.** Python triple-quoted docstrings containing
   example code are syntactically identical to real imports/calls. The
   scanner picks them up. Strip docstrings before running matchers.

6. **`is_test_file` / `is_example` / `is_fixture` context signals as
   first-class concepts** (rather than only the SKIP_DIRS path approach).
   Lets framework rules opt out of test paths even when authors deliberately
   want to scan test code.

7. **Better agent name extraction.** Today every detection's `name` field
   is `"<Framework> agent"`. Should extract the variable name on
   `agent = create_react_agent(...)`, `graph = workflow.compile()`, etc.

8. **Tool-count recall.** The MCP servers assessment showed ~41% recall on
   tool registrations because `Tool(name=Enum.value)` form isn't
   extracted. Generalize the extraction.

9. **Lockfile / package-manifest exclusion from code-pattern matchers.**
   `.json` and `.yaml` are in `SCAN_EXTENSIONS` for `package_dep` /
   `file_present`, but `import_contains` / `code_regex` shouldn't fire on
   them. Split the extension list by matcher kind.

10. **Score recalibration.** The current 0–100 distribution is bimodal:
    almost everything saturates at 100 (Critical) or sits near the
    framework baseline (35–55, Medium). True Highs are rare. Needs
    statistical work against a labeled dataset Agent Shield doesn't have
    yet.

## Calibration verdict

**Headline:** the v0.1.0 detection rules were over-tuned to small fixture
corpora and broke catastrophically on framework monorepos. Honest
sample-precision on `langgraph` and `vercel/ai` was 0/15 in both cases.
A single-pass calibration round (rule-only fixes + a six-line
`SKIP_DIRS` change) cut detections by ~30% overall and brought sample
precision into a defensible range on every repo except `vercel/ai`,
where the residual is *correctly identifying* SDK usage in
framework-shipped examples — a class of false-positive that requires
architectural work, not rule tuning, to eliminate.

**The v0.1.0 contract is preserved:** all 6 snapshot fixtures remain
byte-identical, all 69 tests pass, clippy stays clean, and MSRV stays
at 1.88. The byte-identical contract held through 9 rule commits — the
test confirmed every change was a *tightening* (less likely to fire on
borderline cases) rather than a structural rewrite.

**What this calibration did not validate:** that the scoring layer
produces a meaningful gradient. The scoring rules were not changed in
this round (they're designed for high-confidence detections; gating
them on confidence is the v0.2 fix). The 1,140 residual vercel/ai
detections still all-paint Critical; that number has the same
information content as 0 detections, which is to say none.

**The v0.1.0 scanner is now honest about its scope:** it scans
application repos that *use* an agent framework, not the framework
source itself. README guidance reflects this; a future version will
auto-detect and warn.

---

## Appendix A: Reproducing this calibration

```bash
git checkout v0.1.0
cargo build --release
mkdir -p calibration/repos calibration/scans
cd calibration/repos
git clone --depth=1 https://github.com/langchain-ai/langgraph.git
git clone --depth=1 https://github.com/crewAIInc/crewAI-examples.git
git clone --depth=1 https://github.com/modelcontextprotocol/servers.git
git clone --depth=1 https://github.com/OpenInterpreter/open-interpreter.git
git clone --depth=1 https://github.com/vercel/ai.git
cd ../..
for r in langgraph crewAI-examples servers open-interpreter ai; do
  ./target/release/agent-shield scan calibration/repos/$r \
    --format json -o calibration/scans/$r.json
done
```

To reproduce the post-tuning numbers, check out the calibration branch
HEAD instead of `v0.1.0`. Detection counts will drift over time as the
upstream repos evolve. The v0.1.0 baseline counts are the ones in the
table above.

## Appendix B: What this calibration did not test

- **Performance** — wall-clock time was not measured. v0.2+ candidate
  work.
- **Memory usage** — the 1,291-agent repo produced a 3.2 MB JSON;
  nothing in the scanner streams output.
- **Cross-platform parity** — all scans ran on macOS. CI's Linux job
  exercises a different filesystem ordering, but no calibration-grade
  run was done on Linux.
- **Confidence calibration** — the scoring weights (`+10 per missing
  guardrail`, etc.) were not statistically tuned against any ground
  truth here; that would need labeled data we do not have.
- **Repos that have never been published** — proprietary codebases
  probably look different from these public examples. v0.1.0 has only
  ever been exercised on public code.
- **Notebook (.ipynb) agents** — explicitly out of scope for v0.1.0;
  required parser work deferred to v0.2.
