# Attractor Spec Compliance Review

**Date**: 2026-02-21
**Spec**: `docs/specs/attractor-spec.md`
**Implementation**: `crates/attractor/src/`

---

## Section 1: Overview and Goals

| # | Subsection | Verdict |
|---|-----------|---------|
| 1 | 1.1 Problem Statement | ALIGNED |
| 2 | 1.2 Why DOT Syntax | ALIGNED |
| 3 | 1.3 Design Principles | ALIGNED |
| 4 | 1.4 Layering and LLM Backends | ALIGNED |

**Details**: All five design principles are implemented: declarative pipelines (DOT parsed into `Graph`, engine handles execution), pluggable handlers (`HandlerRegistry`), checkpoint/resume (`checkpoint.rs`), human-in-the-loop (`interviewer/` module), edge-based routing (`select_edge()` in `engine.rs`). `CodergenBackend` trait decouples LLM integration. `EventEmitter` provides the event stream for frontends.

---

## Section 2: DOT DSL Schema

| # | Subsection | Verdict |
|---|-----------|---------|
| 5 | 2.1 Supported Subset | ALIGNED |
| 6 | 2.2 BNF-Style Grammar | ALIGNED |
| 7 | 2.3 Key Constraints | ALIGNED |
| 8 | 2.4 Value Types | ALIGNED |
| 9 | 2.5 Graph Attributes | ALIGNED |
| 10 | 2.6 Node Attributes | ALIGNED |
| 11 | 2.7 Edge Attributes | ALIGNED |
| 12 | 2.8 Shape-to-Handler Mapping | ALIGNED |
| 13 | 2.9 Chained Edges | ALIGNED |
| 14 | 2.10 Subgraphs | ALIGNED |
| 15 | 2.11 Node/Edge Default Blocks | ALIGNED |
| 16 | 2.12 Class Attribute | ALIGNED |
| 17 | 2.13 Minimal Examples | ALIGNED |

**Minor note**: The parser does not produce a specific error message when `strict digraph` is used — it simply fails to parse. Functionally correct but a UX gap for error messaging.

---

## Section 3: Pipeline Execution Engine

| # | Subsection | Verdict |
|---|-----------|---------|
| 18 | 3.1 Run Lifecycle (5 phases) | ALIGNED |
| 19 | 3.2 Core Execution Loop | ALIGNED |
| 20 | 3.3 Edge Selection Algorithm (5-step priority) | ALIGNED |
| 21 | 3.4 Goal Gate Enforcement | ALIGNED |
| 22 | 3.5 Retry Logic | ALIGNED |
| 23 | 3.6 Retry Policies (5 presets) | ALIGNED |
| 24 | 3.7 Failure Routing | ALIGNED |
| 25 | 3.8 Concurrency Model | ALIGNED |

**Minor note**: The 5 named retry presets (none, standard, aggressive, linear, patient) exist as constructors on `RetryPolicy` but `build_retry_policy()` always constructs a custom policy from `max_retries` + default backoff. There is no mechanism for a node to select a preset by name (e.g. `retry_policy="aggressive"`).

---

## Section 4: Node Handlers

| # | Subsection | Verdict |
|---|-----------|---------|
| 26 | 4.1 Handler Interface | ALIGNED |
| 27 | 4.2 Handler Registry | ALIGNED |
| 28 | 4.3 Start Handler | ALIGNED |
| 29 | 4.4 Exit Handler | ALIGNED |
| 30 | 4.5 Codergen Handler | ALIGNED |
| 31 | 4.6 Wait For Human Handler | ALIGNED |
| 32 | 4.7 Conditional Handler | ALIGNED |
| 33 | 4.8 Parallel Handler | ALIGNED |
| 34 | 4.9 Fan-In Handler | ALIGNED |
| 35 | 4.10 Tool Handler | ALIGNED |
| 36 | 4.11 Manager Loop Handler | ALIGNED |
| 37 | 4.12 Custom Handlers | ALIGNED |

**Minor note**: Manager loop reads `stack.child_dotfile` from node attrs rather than graph attrs as the spec pseudocode shows. Arguably better design (per-node child pipeline), but deviates from spec.

---

## Section 5: State and Context

| # | Subsection | Verdict |
|---|-----------|---------|
| 38 | 5.1 PipelineContext | GAP |
| 39 | 5.2 Outcome | ALIGNED |
| 40 | 5.3 Checkpoint | ALIGNED |
| 41 | 5.4 Context Fidelity | GAP |
| 42 | 5.5 Artifact Store | ALIGNED |
| 43 | 5.6 Run Directory Structure | GAP |

**GAP 38 — `last_stage` / `last_response` context keys**: The spec defines these as engine-set context keys. The engine does not set them; only the `codergen` handler sets them. Other handler types do not propagate these keys. Additionally, `internal.retry_count.<node_id>` is tracked in a separate `node_retries` HashMap rather than as a context key.

**GAP 41 — Thread resolution step 3**: The spec lists "Graph-level default thread" as step 3 in thread ID resolution. The implementation uses the node's first CSS class instead. No graph-level default thread concept is implemented.

**GAP 43 — Per-node `prompt.md` / `response.md`**: The spec defines these as part of the run directory structure. The engine does not write them; only the `codergen` handler does. Other LLM-interacting handlers (fan_in with LLM evaluation) do not write these files.

---

## Section 6: Human-in-the-Loop (Interviewer Pattern)

| # | Subsection | Verdict |
|---|-----------|---------|
| 44 | 6.1 Interviewer Interface | ALIGNED |
| 45 | 6.2 Question Model | ALIGNED |
| 46 | 6.3 Answer Model | ALIGNED |
| 47 | 6.4 Built-In Implementations (5) | ALIGNED |
| 48 | 6.5 Timeout Handling | GAP |
| 49 | 6.6 Gate Node Behavior | ALIGNED |

**GAP 48 — WaitHumanHandler bypasses timeout**: `ask_with_timeout()` exists as a utility function but `WaitHumanHandler` calls `interviewer.ask()` directly, so `timeout_seconds` on questions is not enforced for human gate interactions.

---

## Section 7: Validation and Linting

| # | Subsection | Verdict |
|---|-----------|---------|
| 50 | 7.1 Diagnostic Model | ALIGNED |
| 51 | 7.2 Built-In Rules (14 rules) | ALIGNED |
| 52 | 7.3 Validation API | ALIGNED |
| 53 | 7.4 Custom Lint Rules | ALIGNED |

**Details**: All 14 spec rules implemented plus a bonus `direction_valid` rule (15 total). Error-severity diagnostics block execution. Custom rules supported via `extra_rules` parameter.

---

## Section 8: Model Stylesheet

| # | Subsection | Verdict |
|---|-----------|---------|
| 54 | 8.1 Purpose | ALIGNED |
| 55 | 8.2 Grammar | ALIGNED |
| 56 | 8.3 Selectors and Specificity | ALIGNED |
| 57 | 8.4 Recognized Properties | ALIGNED |
| 58 | 8.5 Application/Resolution Order | ALIGNED |

**Details**: Specificity correctly implemented (Universal=0, Class=1, Id=2). Explicit node attributes are never overridden. Stylesheet applied as a transform before validation.

---

## Section 9: Transforms and Extensibility

| # | Subsection | Verdict |
|---|-----------|---------|
| 59 | 9.1 AST Transforms | ALIGNED |
| 60 | 9.2 Built-In Transforms (3) | ALIGNED |
| 61 | 9.3 Custom Transforms | ALIGNED |
| 62 | 9.4 Pipeline Composition | ALIGNED |
| 63 | 9.5 HTTP Server Mode | GAP |
| 64 | 9.6 Observability and Events | ALIGNED |
| 65 | 9.7 Tool Call Hooks | GAP |

**GAP 63 — HTTP Server Mode**: No HTTP server implementation. The spec says "Implementations may expose" making this optional, but it is unimplemented.

**GAP 65 — Tool Call Hooks**: `tool_hooks.pre` and `tool_hooks.post` are defined in the spec for shell commands around LLM tool calls. Not implemented in any handler. Pre-hook should gate tool calls (non-zero exit = skip), post-hook for logging/auditing.

**Minor note**: Transform trait uses `&mut Graph` (in-place mutation) rather than returning a new graph as spec describes. Functionally equivalent.

---

## Section 10: Condition Expression Language

| # | Subsection | Verdict |
|---|-----------|---------|
| 66 | 10.1 Overview | ALIGNED |
| 67 | 10.2 Grammar | ALIGNED |
| 68 | 10.3 Semantics | ALIGNED |
| 69 | 10.4 Variable Resolution | ALIGNED |
| 70 | 10.5 Evaluation | ALIGNED |
| 71 | 10.6 Examples | ALIGNED |
| 72 | 10.7 Extended Operators (future) | ALIGNED |

**Details**: Full implementation with `=`, `!=`, `&&` conjunction, bare key truthiness, `context.*` double-lookup. Correctly does not implement future operators.

---

## Section 11: Definition of Done

| # | Subsection | Verdict |
|---|-----------|---------|
| 73 | 11.1 DOT Parsing | ALIGNED |
| 74 | 11.2 Validation and Linting | ALIGNED |
| 75 | 11.3 Execution Engine | ALIGNED |
| 76 | 11.4 Goal Gate Enforcement | ALIGNED |
| 77 | 11.5 Retry Logic | ALIGNED |
| 78 | 11.6 Node Handlers | ALIGNED |
| 79 | 11.7 State and Context | ALIGNED |
| 80 | 11.8 Human-in-the-Loop | ALIGNED |
| 81 | 11.9 Condition Expressions | ALIGNED |
| 82 | 11.10 Model Stylesheet | ALIGNED |
| 83 | 11.11 Transforms and Extensibility | ALIGNED |
| 84 | 11.12 Cross-Feature Parity Matrix | ALIGNED |
| 85 | 11.13 Integration Smoke Test | ALIGNED |

---

## Summary

| Category | Count |
|----------|-------|
| Total items reviewed | 85 |
| ALIGNED | 78 |
| GAP | 7 |
| Alignment rate | 91.8% |

### All Gaps

| # | Section | Gap | Severity |
|---|---------|-----|----------|
| 38 | 5.1 | `last_stage`/`last_response` not set by engine; `internal.retry_count` not in context | Low |
| 41 | 5.4 | Thread resolution missing graph-level default thread (step 3) | Low |
| 43 | 5.6 | `prompt.md`/`response.md` only written by codergen, not other LLM handlers | Low |
| 48 | 6.5 | WaitHumanHandler calls `ask()` directly, bypassing `ask_with_timeout()` | Medium |
| 63 | 9.5 | HTTP server mode not implemented (spec marks as optional) | Low |
| 65 | 9.7 | `tool_hooks.pre`/`tool_hooks.post` not implemented | Medium |

### Minor Notes (not gaps, but deviations)

- No specific error for `strict digraph` (parser just fails)
- Named retry presets exist but no node-level attribute to select them
- Manager loop reads `child_dotfile` from node attrs not graph attrs
- Transform trait mutates in-place rather than returning new graph
