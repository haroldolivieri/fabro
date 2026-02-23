# Attractor Spec: Confirmed Gaps Report

**Date:** 2026-02-21
**Method:** 3 parallel agents investigated 19 claimed gaps, reading source code and citing exact file:line evidence.

**Result: 16 CONFIRMED / 2 REFUTED / 1 PARTIAL**

---

## Hard Gaps (3/3 confirmed)

### 1. Auto Status — engine never checks `auto_status` attribute

**CONFIRMED**

The `auto_status()` accessor exists at `graph/types.rs:193-194` but is never referenced in `engine.rs` or any handler. A grep for `auto_status` across all source returns only the accessor definition and its unit test — zero engine references. The spec (line 162, Appendix C line 2111) says: when `auto_status=true` and no `status.json` was written by the handler, the engine should synthesize `{"outcome": "success", "notes": "auto-status: handler completed without writing status"}`. This does not happen.

### 2. Timeout Enforcement — no deadline around handler execution

**CONFIRMED**

`tokio::time::timeout` appears in exactly two places: `handler/tool.rs:67` (subprocess timeout) and `interviewer/mod.rs:142` (human input timeout). The engine's `execute_with_retry` at `engine.rs:449-536` wraps handler execution in `catch_unwind` for panic safety (line 464) but has **no** `tokio::time::timeout` wrapper. A grep for `timeout` in `engine.rs` returns zero matches. All non-tool, non-interviewer handlers (codergen, manager_loop, parallel, etc.) can run indefinitely.

### 3. Manager Loop `child_autostart` — not implemented

**CONFIRMED**

The spec (lines 961-963) says: `IF node.attrs.get("stack.child_autostart", "true") == "true": start_child_pipeline(child_dotfile)`. A codebase-wide grep for `child_autostart`, `start_child_pipeline`, and `child_dotfile` returns matches only in the spec itself and the review document. `handler/manager_loop.rs` goes directly into its observation loop without any auto-start logic. The `ChildObserver` trait (lines 16-22) provides `observe` and `steer` but no launch capability. Sub-gap: `steer_cooldown_elapsed()` is also missing — zero matches for `steer_cooldown` anywhere.

---

## Minor Gaps (13/16 confirmed, 2 refuted, 1 partial)

### 4. Thread ID Resolution — only step 1 of 5

**CONFIRMED**

The spec (lines 1196-1206) defines 5-step thread resolution for `full` fidelity. `engine.rs:660-664` only handles step 1 (node `thread_id`). Missing:
- Step 2: Edge `thread_id` — accessor exists at `graph/types.rs:262-264` but engine never reads it
- Step 3: Graph-level default thread — no `default_thread` accessor (zero grep matches)
- Step 4: Derived class from enclosing subgraph — engine never uses `classes` for thread resolution
- Step 5: Fallback to previous node ID — not implemented

### 5. Checkpoint Resume — retry counters not restored

**CONFIRMED**

`Checkpoint.node_retries` field exists at `checkpoint.rs:17` but `Checkpoint::from_context()` at line 28 always initializes it as `HashMap::new()`. A grep for `node_retries` in `engine.rs` returns zero matches. The field is never populated during saves and never read during resume. The spec's `reset_retry_counter`/`increment_retry_counter` functions (lines 499-504) do not exist.

### 6. Checkpoint Resume — fidelity degradation missing

**CONFIRMED**

The spec (line 1165) says: "If the previous node used `full` fidelity, degrade to `summary:high` for the first resumed node." A grep for `fidelity.*degrad|summary.high.*resume` across all sources returns zero matches. The resume code at `engine.rs:591-608` performs no fidelity degradation.

### 7. ~~Retry Policy `should_retry` — too coarse~~ — RESOLVED

`AttractorError::is_retryable()` classifies errors by variant (`Handler`/`Engine`/`Io` = retryable; `Parse`/`Validation`/`Stylesheet`/`Checkpoint`/`Cancelled` = terminal). The `Handler` trait now has a `should_retry(&self, err: &AttractorError) -> bool` method (default delegates to `is_retryable()`). The engine's `execute_with_retry` calls the handler method directly. Handlers can override to customize retry behavior.

### 8. Direction type not validated

**CONFIRMED**

The BNF defines `Direction ::= 'TB' | 'LR' | 'BT' | 'RL'` (spec line 107). `grammar.rs:57-66` accepts any `identifier = value` as a graph attr declaration. `semantic.rs:178-179` inserts without validation. None of the 14 validation rules check direction values. `rankdir=XY` would be accepted silently.

### 9. Stylesheet `stylesheet_syntax` lint — brace balance only

**CONFIRMED**

`validation/rules.rs:320-348`: the rule counts `{` and `}` characters and errors only if counts differ. It does not call `parse_stylesheet()` from `stylesheet.rs:54-85` which performs full parsing (selector validation, declaration parsing, proper error messages). A stylesheet like `* { garbage garbage }` passes the lint.

### 10. Stylesheet — undocumented Shape selector

**CONFIRMED**

The spec grammar (line 1497) defines: `Selector ::= '*' | '#' Identifier | '.' ClassName` — three types. `stylesheet.rs:6-15` defines four: `Universal`, `Shape(String)`, `Class(String)`, `Id(String)`. The `Shape` selector is parsed at lines 117-131 (bare-word fallback). This shifts specificity: spec says `*`=0, `.class`=1, `#id`=2; impl has `*`=0, `Shape`=1, `.class`=2, `#id`=3. Relative ordering preserved but absolute values differ. Test at line 446 confirms `box { llm_model: opus; }` produces `Selector::Shape("box")`.

### 11. Fan-in — no score sort, all-fail returns SUCCESS

**CONFIRMED** (both sub-claims)

The spec (line 919) sorts by `(outcome_rank, -c.score, c.id)`. `fan_in.rs:103-109` sorts by `(status_rank, id)` — no `score` field on `Candidate` (lines 62-65), zero grep matches for "score". For all-fail: the spec (line 923) says "Only when all candidates fail does fan-in return FAIL." `fan_in.rs:41-58` always builds `Outcome::success()` at line 47 regardless of whether all candidates failed.

### 12. Preamble transform — applied at parse time, not execution time

**CONFIRMED**

The spec (line 1602): "Applied at execution time (not at parse time) since it depends on runtime state." `pipeline.rs:39` calls `PreambleTransform.apply(&mut graph)` in `prepare()` alongside other parse-time transforms. `transform.rs:28-49` reads `fidelity` from static node attributes but has no access to runtime state (e.g., edge-level fidelity overrides resolved at execution time in `engine.rs:656`). If a node has `fidelity="full"` but an incoming edge has `fidelity="truncate"`, the preamble would be incorrectly missing.

### 13. Pre-hook non-zero — returns fail instead of skip

**CONFIRMED**

The spec (line 1693): "non-zero means skip the tool call." `codergen.rs:98-103` returns `Outcome::fail("pre-hook failed, skipping LLM call")` — `StageStatus::Fail` is semantically stronger than skipping. Test at line 302 confirms the outcome status is `Fail`.

### 14. No parallel fan-out/fan-in integration test

**CONFIRMED**

`tests/integration.rs` (1181 lines) contains 11+ test functions covering linear, branching, human gate, goal gate, retry, stylesheet, checkpoint, and smoke test pipelines. None involve `component` (parallel) or `tripleoctagon` (fan-in) shape nodes. A grep for `parallel|fan_in|fan_out` in `crates/attractor/tests/` returns zero matches.

### 15. Manifest missing `goal` field

**CONFIRMED**

The spec (line 1260): "manifest.json -- Pipeline metadata (name, goal, start time)." `engine.rs:217-233` `write_manifest()` writes `pipeline_name`, `start_time`, `node_count`, `edge_count` — no `goal` field despite `graph.goal()` being available. Integration test at lines 1527-1532 confirms the four fields without `goal`.

### 16. ~~Error categories — no retryable/terminal classification~~ — RESOLVED

`AttractorError::is_retryable()` at `error.rs:38` classifies variants as retryable (`Handler`, `Engine`, `Io`) or terminal (`Parse`, `Validation`, `Stylesheet`, `Checkpoint`, `Cancelled`). The `Handler` trait's `should_retry` default impl delegates to this method. No `ErrorCategory` enum, but the classification is functionally equivalent.

### 17. Spec self-contradicts on `default_max_retry`

**CONFIRMED**

Spec line 138 (Section 2.5 table): `default_max_retry | Integer | 50`. Spec line 481 (Section 3.5): "Built-in default: 0 (no retries)." Implementation follows Section 2 at `graph/types.rs:349-353` with `.unwrap_or(50)`. Test at `engine.rs:931-936` confirms 51 max attempts (50 retries + 1 initial).

---

## Refuted Claims (2)

### R1. Missing variable handling — claimed as GAP

**REFUTED** — Correctly implemented.

`condition.rs:87-110` `resolve_key()` returns `String::new()` for all missing keys (lines 105, 109). Tests at lines 230-239 (`missing_key_compares_as_empty`) and 291-295 (`bare_key_falsy_when_empty`) confirm spec-compliant behavior. The spec (line 1724) says: "Missing keys compare as empty strings" — exactly what the implementation does.

### R2. Status File Contract — claimed "no implementation found"

**REFUTED** — Implemented at two layers.

**Engine-level:** `engine.rs:236-248` `write_node_status()` writes `{node_id}/status.json` with `status`, `notes`, `failure_reason`, `timestamp`. Called at line 702-703 for every node.
**Handler-level:** `codergen.rs:111,150` writes richer `status.json` (full `Outcome` serialized).
**Tests:** `engine.rs:1536-1551` and `integration.rs:189-198` verify status file existence and contents.

Minor sub-gap: the engine-level schema uses key `"status"` while the spec's Appendix C uses `"outcome"`, and the engine-level file omits `preferred_next_label`, `suggested_next_ids`, `context_updates`.

---

## Partial (1)

### P1. Checkpoint Resume — functional but incomplete

**PARTIAL**

`engine.rs:554-561` `run_from_checkpoint()` exists and is callable. Resume logic at lines 591-608 restores context, logs, completed_nodes, and continues from the next node.

**What works:** Basic resume for simple linear pipelines.

**What's missing:**
1. `node_retries` ignored during resume (see gap 5)
2. `node_outcomes` not restored — initialized as empty HashMap at line 586, causing goal gate checks to miss pre-checkpoint outcomes
3. Edge selection during resume picks first outgoing edge (line 603-608), ignoring conditions — wrong successor for conditional graphs
4. No integration test calls `run_from_checkpoint` — only `Checkpoint::save`/`load` is tested

---

## Summary Table

| # | Gap | Verdict | Severity |
|---|-----|---------|----------|
| 1 | Auto status not enforced | CONFIRMED | Hard |
| 2 | Timeout not enforced in engine | CONFIRMED | Hard |
| 3 | Manager loop child_autostart missing | CONFIRMED | Hard |
| 4 | Thread ID resolution 1/5 steps | CONFIRMED | Moderate |
| 5 | Checkpoint retry counters not persisted | CONFIRMED | Moderate |
| 6 | Checkpoint fidelity degradation missing | CONFIRMED | Moderate |
| 7 | ~~should_retry retries all errors~~ | RESOLVED | ~~Moderate~~ |
| 8 | Direction values not validated | CONFIRMED | Low |
| 9 | Stylesheet lint brace-balance only | CONFIRMED | Low |
| 10 | Undocumented Shape selector | CONFIRMED | Low |
| 11 | Fan-in: no score sort, all-fail=SUCCESS | CONFIRMED | Moderate |
| 12 | Preamble at parse time not runtime | CONFIRMED | Moderate |
| 13 | Pre-hook fail instead of skip | CONFIRMED | Low |
| 14 | No parallel integration test | CONFIRMED | Low |
| 15 | Manifest missing goal field | CONFIRMED | Low |
| 16 | ~~No error retryable/terminal classification~~ | RESOLVED | ~~Moderate~~ |
| 17 | Spec contradicts itself on default_max_retry | CONFIRMED | Low (spec bug) |
| P1 | Checkpoint resume incomplete | PARTIAL | Moderate |
| R1 | Missing variable handling | REFUTED | — |
| R2 | Status file contract | REFUTED | — |
