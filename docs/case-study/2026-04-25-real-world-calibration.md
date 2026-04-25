# Real-World Calibration: Agent Shield v0.1.0 Against 5 OSS Agent Repos

**Date:** 2026-04-25
**Scanner version:** 0.1.0 (commit `78da44a`)
**Status:** DRAFT — agent reports incoming, will be synthesized into final findings

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
   it got right, and what should change for v0.2. Written for project
   contributors and the maintainer's future self.
2. **A site-ready marketing excerpt** (`site/docs/case-study-real-world.html`)
   — the polished, externally-presentable version. Same data, different
   audience.

## Methodology

### Repo selection (5 repos, diverse frameworks)

| # | Repo | Framework focus | Rationale |
|---|---|---|---|
| 1 | `langchain-ai/langgraph` | LangGraph (official) | Framework's own examples — accuracy floor |
| 2 | `crewAIInc/crewAI-examples` | CrewAI (official-adjacent) | Canonical multi-agent CrewAI usage |
| 3 | `modelcontextprotocol/servers` | Anthropic MCP (official) | Reference MCP server implementations |
| 4 | `OpenInterpreter/open-interpreter` | Custom agent (3rd-party) | Popular agent CLI; tests "Custom Agent" rule |
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
  Agent" path.
- **vercel/ai at 1,291** — this is the AI SDK *framework repo itself*. The
  scanner is happily detecting agents inside the implementation of the
  framework, including tests and type definitions. Whether that's "wrong" is
  a more interesting question than it looks; see the vercel/ai section.

The other three numbers are plausible and need per-detection inspection
before any verdict.

## Per-repo findings

> **Status:** Each section below will be filled in from a parallel subagent
> assessment. The agents are scanning real files and reading the YAML rules
> directly, so the recommendations are concrete patches, not speculation.

### langchain-ai/langgraph (385 detections)

_(awaiting agent)_

### crewAIInc/crewAI-examples (82 detections)

_(awaiting agent)_

### modelcontextprotocol/servers (59 detections)

_(awaiting agent)_

### OpenInterpreter/open-interpreter (0 detections — INVESTIGATE)

_(awaiting agent — diagnostic mission)_

### vercel/ai (1,291 detections — INVESTIGATE)

_(awaiting agent — over-detection root-cause)_

## Synthesis

_(filled in after all 5 agent reports land)_

## Rule fixes applied in this calibration

_(commit-by-commit list of every rule edit, with before/after)_

## What did not get fixed (deferred to v0.2)

_(judgment-call items where the right answer needs more discussion)_

## Calibration verdict

_(one-paragraph honest assessment of the v0.1.0 scanner against real code)_

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

Detection counts will drift over time as the upstream repos evolve. The
v0.1.0 baseline counts are the ones in the table above.

## Appendix B: What this calibration did not test

- **Performance** — wall-clock time was not measured. v0.2+ candidate work.
- **Memory usage** — the 1,291-agent repo produced a 3.2 MB JSON; nothing in
  the scanner streams output.
- **Cross-platform parity** — all scans ran on macOS. CI's Linux job exercises
  a different filesystem ordering, but no calibration-grade run was done on
  Linux.
- **Confidence calibration** — the scoring weights (`+10 per missing
  guardrail`, etc.) were not statistically tuned against any ground truth
  here; that would need labeled data we do not have.
- **Repos that have never been published** — proprietary codebases probably
  look different from these public examples. v0.1.0 has only ever been
  exercised on public code.
