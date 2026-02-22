# Attractor Spec Compliance Review

**Date:** 2026-02-21
**Spec:** `docs/specs/attractor-spec.md`
**Implementation:** `crates/attractor/src/`
**Reviewers:** 5 parallel agents, each covering distinct spec sections

---

## Section 1: Overview and Goals

### 1. Section 1.1 — Problem Statement
**ALIGNED**
The implementation delivers a DOT-based directed-graph pipeline runner as described. No code artifact required beyond the overall architecture.

### 2. Section 1.2 — Why DOT Syntax
**ALIGNED**
The parser (`parser/grammar.rs`) starts with `digraph` keyword and builds on directed graph primitives. DOT subset parser implemented from scratch.

### 3. Section 1.3 — Design Principles
**ALIGNED**
- Declarative pipelines: `.dot` files declare graph structure; engine traverses it (`parser/mod.rs:18`, `graph/types.rs:277-294`)
- Pluggable handlers: `Node::handler_type()` at `graph/types.rs:203-209` resolves from `type` attr or shape mapping
- Checkpoint and resume: `checkpoint` module (`lib.rs:2`), checkpoint save/load implemented
- Human-in-the-loop: `interviewer` module (`lib.rs:10`), `wait.human` mapped at `graph/types.rs:77`
- Edge-based routing: `Edge` struct has `condition()`, `weight()`, `label()` accessors at `graph/types.rs:241-274`

### 4. Section 1.4 — Layering and LLM Backends
**ALIGNED**
No LLM SDK dependency. `CodergenBackend` trait decouples LLM calls. Event stream module at `lib.rs:8`.

---

## Section 2: DOT DSL Schema

### 5. Section 2.1 — Supported Subset
**ALIGNED**
Parser accepts only `digraph` (`grammar.rs:140`). No code path for `graph` (undirected) or `strict`. Trailing content rejected at `parser/mod.rs:23-29`.

### 6. Section 2.2 — BNF Grammar
**ALIGNED** (minor gap)
All grammar productions verified against implementation:
- `Graph`, `Statement`, `GraphAttrStmt`, `NodeDefaults`, `EdgeDefaults`, `GraphAttrDecl`, `SubgraphStmt`, `NodeStmt`, `EdgeStmt`, `AttrBlock`, `Attr` — all match at `grammar.rs:14-152`
- `QualifiedId` (dotted keys) supported at `lexer.rs:90-113`
- All value types including Duration with `ms/s/m/h/d` at `lexer.rs:229-246`
- **Minor gap:** `Direction` type (`TB|LR|BT|RL`) parsed as bare `AstValue::Ident` — no validation restricting to valid values. Functionally works but invalid directions accepted silently.

### 7. Section 2.3 — Key Constraints
**ALIGNED**
- One digraph per file: trailing content check at `parser/mod.rs:23-29`
- Bare identifiers: `[A-Za-z_][A-Za-z0-9_]*` at `lexer.rs:82-87`
- Commas required: `separated_list1(preceded(ws, char(',')), attr)` at `grammar.rs:26-31`
- Directed edges only: only `->` parsed at `lexer.rs:262-264`
- Comments: `strip_comments()` at `lexer.rs:5-53` handles `//` and `/* */`
- Semicolons optional: `opt_semi` at `grammar.rs:34-36`

### 8. Section 2.4 — Value Types
**ALIGNED**
All five types implemented with correct syntax:
| Type | Implementation |
|------|---------------|
| String | `lexer.rs:121-177` with `\"`, `\n`, `\t`, `\\` escapes |
| Integer | `lexer.rs:208-226` with sign and float-rejection |
| Float | `lexer.rs:193-205` |
| Boolean | `lexer.rs:180-190` |
| Duration | `lexer.rs:229-246` with `ms/s/m/h/d` units |

### 9. Section 2.5 — Graph-Level Attributes
**ALIGNED**
All 7 attributes present with correct defaults:
| Key | Evidence |
|-----|----------|
| `goal` | `graph/types.rs:333-338` — default `""` |
| `label` | stored as generic attr |
| `model_stylesheet` | `graph/types.rs:341-346` — default `""` |
| `default_max_retry` | `graph/types.rs:349-354` — default `50` |
| `retry_target` | `graph/types.rs:357-359` |
| `fallback_retry_target` | `graph/types.rs:361-366` |
| `default_fidelity` | `graph/types.rs:369-373` |

### 10. Section 2.6 — Node Attributes
**ALIGNED**
All 18 attributes present with correct defaults:
| Key | Evidence |
|-----|----------|
| `label` | `graph/types.rs:119-121` — falls back to node ID |
| `shape` | `graph/types.rs:124-126` — default `"box"` |
| `type` | `graph/types.rs:129-131` |
| `prompt` | `graph/types.rs:134-136` |
| `max_retries` | `graph/types.rs:139-141` — `None` when unset |
| `goal_gate` | `graph/types.rs:144-146` — default `false` |
| `retry_target` | `graph/types.rs:149-151` |
| `fallback_retry_target` | `graph/types.rs:153-156` |
| `fidelity` | `graph/types.rs:159-161` |
| `thread_id` | `graph/types.rs:164-166` |
| `class` | `graph/types.rs:169-171` + parsing at `semantic.rs:103-117` |
| `timeout` | `graph/types.rs:173-175` — `Option<Duration>` |
| `llm_model` | `graph/types.rs:178-180` |
| `llm_provider` | `graph/types.rs:183-185` |
| `reasoning_effort` | `graph/types.rs:188-190` — default `"high"` |
| `auto_status` | `graph/types.rs:193-195` — default `false` |
| `allow_partial` | `graph/types.rs:198-200` — default `false` |

### 11. Section 2.7 — Edge Attributes
**ALIGNED**
All 7 attributes present:
| Key | Evidence |
|-----|----------|
| `label` | `graph/types.rs:242-244` |
| `condition` | `graph/types.rs:247-249` |
| `weight` | `graph/types.rs:252-254` — default `0` |
| `fidelity` | `graph/types.rs:257-259` |
| `thread_id` | `graph/types.rs:262-264` |
| `loop_restart` | `graph/types.rs:267-269` — default `false` |
| `freeform` | `graph/types.rs:272-274` — default `false` |

### 12. Section 2.8 — Shape-to-Handler Mapping
**ALIGNED**
All 9 mappings present at `graph/types.rs:72-85`. Handler resolution with `type` override at `graph/types.rs:203-209`.

### 13. Section 2.9 — Chained Edges
**ALIGNED**
Parsed at `grammar.rs:86-101`, expanded via `windows(2)` at `semantic.rs:132-141`. Test at `semantic.rs:471-484` confirms correct desugaring.

### 14. Section 2.10-2.12 — Subgraphs, Defaults, Class Attribute
**ALIGNED**
- Subgraph scoping: `semantic.rs:145-235` saves/restores defaults per scope
- Class derivation from subgraph label: `semantic.rs:51-58` (`derive_class_from_label`)
- Comma-separated class parsing: `semantic.rs:103-117`, tested at `semantic.rs:487-500`
- Node/edge default blocks: `grammar.rs:44-54`, applied in `semantic.rs:76-83`

---

## Section 3: Pipeline Execution Engine

### 15. Section 3.1 — Run Lifecycle
**ALIGNED**
Five phases implemented:
- PARSE: `pipeline.rs:34` calls `crate::parser::parse(dot_source)`
- VALIDATE: `pipeline.rs:46` calls `validation::validate(&graph, &[])`
- INITIALIZE: `engine.rs:580-625` creates run directory, initializes context
- EXECUTE: `engine.rs:627-771` main loop
- FINALIZE: `engine.rs:773-784` emits `PipelineCompleted`, returns outcome

### 16. Section 3.2 — Core Execution Loop
**ALIGNED**
All 8 steps from spec implemented:
1. Start node resolution: `engine.rs:620-624` via `graph.find_start_node()` at `graph/types.rs:310-320`
2. Terminal check: `engine.rs:633-653`
3. Execute with retry: `engine.rs:669-679`
4. Record completion: `engine.rs:706-708`
5. Apply context updates: `engine.rs:711-715`
6. Save checkpoint: `engine.rs:718-730`
7. Select next edge: `engine.rs:733-753`
8. Loop restart: `engine.rs:760-767`, advance: `engine.rs:768`

### 17. Section 3.3 — Edge Selection Algorithm
**ALIGNED**
Five-step priority fully implemented at `engine.rs:299-356`:
1. Condition matching: `engine.rs:311-321`
2. Preferred label: `engine.rs:324-333` with `normalize_label` at `engine.rs:254-279`
3. Suggested next IDs: `engine.rs:336-342`
4. Weight + lexical tiebreak: `engine.rs:345-352` via `best_by_weight_then_lexical` at `engine.rs:282-295`
5. Fallback: `engine.rs:355`

### 18. Section 3.4 — Goal Gate Enforcement
**ALIGNED**
- `check_goal_gates` at `engine.rs:362-377`: checks goal_gate nodes for SUCCESS/PARTIAL_SUCCESS
- `get_retry_target` at `engine.rs:380-408`: four-level fallback chain (node → node fallback → graph → graph fallback)
- `is_terminal` at `engine.rs:411-414`

### 19. Section 3.5 — Retry Logic
**ALIGNED** (minor gaps)
- `build_retry_policy` at `engine.rs:178-189`: node `max_retries` with `default_max_retry` fallback
- `execute_with_retry` at `engine.rs:449-536`: full retry loop
- **Minor:** Retry counters not persisted to `Checkpoint.node_retries` during execution (field exists at `checkpoint.rs:17` but not populated)
- **Minor:** Spec self-contradicts on `default_max_retry` default (section 3.5 says 0, section 2 says 50). Implementation follows section 2 (50).

### 20. Section 3.6 — Retry Policy / Backoff
**ALIGNED** (minor gap)
- `BackoffConfig` at `engine.rs:28-44` with correct defaults
- `delay_for_attempt` at `engine.rs:50-76`: formula matches spec exactly
- All 5 presets match spec table (none/standard/aggressive/linear/patient)
- **Minor:** Default `should_retry` at `engine.rs:101-103` retries ALL errors. Spec defines granular behavior (retry 429/5xx, fail 401/403/400).

### 21. Section 3.7 — Failure Routing
**ALIGNED**
All 4 priority steps at `engine.rs:733-753`: fail edge → retry_target → fallback_retry_target → pipeline termination.

### 22. Section 3.8 — Concurrency Model
**ALIGNED**
Single-threaded graph traversal at `engine.rs:627-771`. One node at a time.

### 23. Section 3.9 — Auto Status
**GAP**
`auto_status()` accessor exists at `graph/types.rs:193-194` but **engine never references it**. Engine always writes status.json itself at line 703. No auto-synthesis logic for when a handler writes no status.

### 24. Section 3.10 — Timeout Enforcement
**GAP**
Node `timeout` attribute parsed as `Option<Duration>` (item 10) but **not enforced during handler execution** in the engine. The tool handler (`tool.rs:61-78`) does use timeout for subprocess execution, but the engine does not wrap general handler execution with a timeout.

---

## Section 4: Node Handlers

### 25. Section 4.1 — Handler Trait
**ALIGNED**
Trait at `handler/mod.rs:22-31`: `async fn execute(&self, node, context, graph, logs_root) -> Result<Outcome>`. All four parameters match spec. `Result` wrapping for error propagation.

### 26. Section 4.2 — Handler Registry
**ALIGNED**
At `handler/mod.rs:34-74`: `HashMap<String, Box<dyn Handler>>`, `default_handler`, `register()` replaces existing, three-step `resolve()` (explicit type → shape → default). Tests confirm at lines 153-177.

### 27. Section 4.3 — Start Handler
**ALIGNED**
`handler/start.rs:13-26`: returns `Outcome::success()`. No-op.

### 28. Section 4.4 — Exit Handler
**ALIGNED**
`handler/exit.rs:13-26`: returns `Outcome::success()`. No goal gate logic (handled by engine).

### 29. Section 4.5 — Codergen Handler
**ALIGNED**
- Prompt building: `codergen.rs:87-91` — `node.prompt()` falling back to `node.label()`
- `$goal` expansion: `codergen.rs:42-44`
- Log writing: `codergen.rs:94-96` — `prompt.md`, `response.md`, `status.json`
- Backend call: `codergen.rs:106-121` — handles `CodergenResult::Full` and `CodergenResult::Text`
- Simulation mode when no backend: `codergen.rs:122+`
- Context updates `last_stage`/`last_response`: `codergen.rs:139-146`
- Tool hooks (pre/post): `codergen.rs:98-131` (enhancement beyond spec)
- `CodergenBackend` trait at `codergen.rs:20-27` matches spec's `run(node, prompt, context) -> String | Outcome`

### 30. Section 4.6 — Wait Human Handler
**ALIGNED**
Comprehensive implementation at `wait_human.rs`:
- Choice derivation from outgoing edges: lines 110-129
- Freeform edge detection: line 115
- No-edges failure: lines 131-133
- Question building with `MultipleChoice`, `allow_freeform`, `stage`: lines 136-150
- Timeout handling with default choice fallback: lines 162-179
- Skipped handling: lines 183-185
- Fixed-choice match with `suggested_next_ids` and context updates: lines 195-201
- Freeform fallback: lines 204-221
- First-choice fallback: lines 224-226
- Accelerator key parsing `[K] Label`, `K) Label`, `K - Label`, first char: lines 32-71

### 31. Section 4.7 — Conditional Handler
**ALIGNED**
`handler/conditional.rs:14-28`: no-op returning SUCCESS with note. Routing handled by engine.

### 32. Section 4.8 — Parallel Handler
**ALIGNED**
Full implementation at `parallel.rs`:
- 4 join policies: `wait_all`, `first_success`, `k_of_n(K)`, `quorum(fraction)` at lines 36-59
- 3 error policies: `continue`, `fail_fast`, `ignore` at lines 62-75
- `max_parallel` with `Semaphore` for bounded concurrency: lines 113-120
- Context isolation via `clone_context()`: line 126
- Join evaluation with all four policies: lines 249-286
- Fail-fast behavior: lines 185-189
- Context storage for fan-in (`parallel.results`, `parallel.branch_count`): lines 230-241

### 33. Section 4.9 — Fan-In Handler
**ALIGNED** (minor gaps)
- Reads `parallel.results`: `fan_in.rs:34-37`
- LLM-based evaluation when prompt + backend present: lines 41-42
- Heuristic selection by status rank: lines 77-115
- Context updates `parallel.fan_in.best_id`/`best_outcome`: lines 48-55
- **Minor:** No `score`-based sorting in heuristic (spec mentions `-c.score`)
- **Minor:** Returns SUCCESS when results exist even if all candidates failed

### 34. Section 4.10 — Tool Handler
**ALIGNED**
At `tool.rs`:
- Reads `tool_command` from attrs: lines 51-55
- Empty command returns fail: lines 57-59
- Runs via `sh -c` with timeout support: lines 61-78
- Sets `tool.output` to stdout: lines 15-40

### 35. Section 4.11 — Manager Loop Handler
**GAP** (partial)
Core observe/steer/wait cycle implemented at `manager_loop.rs:59-157`:
- Poll interval, max cycles, stop condition, actions parsing: lines 67-100
- Observe/steer delegation: lines 105-115
- Child status check: lines 119-132
- Max cycles exceeded: lines 152-155
- **GAP: `stack.child_autostart`/`start_child_pipeline` not implemented** — spec says autostart child pipeline, impl does not
- **Minor:** `steer_cooldown_elapsed()` not implemented — steers every cycle

### 36. Section 4.12 — Custom Handlers
**ALIGNED**
Trait-based design inherently supports custom handlers via `register()`. `Send + Sync` bounds match spec contract.

---

## Section 5: State and Context

### 37. Section 5.1 — Context
**ALIGNED**
Thread-safe key-value store at `context.rs:1-11`: `Arc<RwLock<HashMap<String, Value>>>` with all spec methods:
- `set()`: line 33-38
- `get()`: line 46-52
- `get_string()`: line 56-60
- `append_log()`: line 67-72
- `snapshot()`: line 80-85
- `clone_context()`: line 98-106 (deep copy for parallel isolation)
- `apply_updates()`: line 113-118

### 38. Section 5.2 — Outcome
**ALIGNED**
At `outcome.rs`:
- `StageStatus` enum (lines 10-16): `Success`, `Fail`, `PartialSuccess`, `Retry`, `Skipped` — all 5 spec variants
- `Outcome` struct (lines 48-60): `status`, `preferred_label`, `suggested_next_ids`, `context_updates`, `notes`, `failure_reason` — all match
- Factory methods (lines 63-107): `success()`, `fail()`, `retry()`, `skipped()`

### 39. Section 5.3 — Checkpoint
**ALIGNED** (minor gaps)
At `checkpoint.rs:13-20`: `timestamp`, `current_node`, `completed_nodes`, `node_retries`, `context_values`, `logs` — all match spec.
- `save()` at line 44-49, `load()` at line 56-61
- Resume at `engine.rs:591-608`: restores context, logs, completed_nodes, resumes from next node
- **Minor:** Retry counters not restored during resume
- **Minor:** Fidelity degradation on resume (`full` → `summary:high`) not implemented per spec line 1166

### 40. Section 5.4 — Fidelity Modes
**ALIGNED** (minor gap)
`resolve_fidelity` at `engine.rs:199-211` implements full 4-level precedence:
1. Edge `fidelity` attribute (line 201)
2. Target node `fidelity` attribute (line 205)
3. Graph `default_fidelity` attribute (line 208)
4. Default: `"compact"` (line 211)

Tests confirm all four levels at `engine.rs:1462-1511`.

Thread tracking: `engine.rs:660-664` stores `thread.{tid}.current_node`.

**Minor gap:** Thread ID resolution only implements step 1 of 5 from spec (node `thread_id`). Missing: edge `thread_id`, graph default, subgraph class derivation, fallback to previous node ID.

### 41. Section 5.5 — Artifact Store
**ALIGNED**
Full implementation at `artifact.rs`:
- `ArtifactStore` with `base_dir` and `RwLock<HashMap>`: lines 31-34
- `store()` with file-backing above 100KB threshold: lines 62-96
- `retrieve()` from memory or file: lines 107-130
- `has()`, `list()`, `remove()`, `clear()`: lines 137-184
- `ArtifactInfo` struct with all 5 fields: lines 16-22

### 42. Section 5.6 — Run Directory Structure
**ALIGNED** (minor gap)
All spec artifacts written: `checkpoint.json`, `manifest.json`, `{node_id}/status.json`, `{node_id}/prompt.md`, `{node_id}/response.md`, `artifacts/{artifact_id}.json`.
- **Minor:** Manifest (`engine.rs:217-232`) includes `node_count`/`edge_count` but not `goal` field from spec.

---

## Section 6: Human-in-the-Loop (Interviewer Pattern)

### 43. Section 6.1 — Interviewer Interface
**ALIGNED**
Three methods at `interviewer/mod.rs:152-167`: `ask()`, `ask_multiple()` (with default sequential impl), `inform()` (with default no-op). Async via `async_trait`.

### 44. Section 6.2 — Question Model
**ALIGNED**
At `mod.rs:29-39`: `text`, `question_type` (4 variants: `YesNo`, `MultipleChoice`, `Freeform`, `Confirmation`), `options` (key + label), `allow_freeform`, `default`, `timeout_seconds`, `stage`, `metadata` — all 8 fields present.

### 45. Section 6.3 — Answer Model
**ALIGNED**
`AnswerValue` enum (lines 57-65): `Yes`, `No`, `Skipped`, `Timeout`, `Selected(String)`, `Text(String)`. `Answer` struct (lines 68-73): `value`, `selected_option`, `text`.

### 46. Section 6.4 — Built-In Interviewers
**ALIGNED** (all 5)
- **AutoApprove** (`auto_approve.rs:10-23`): YES for YesNo/Confirmation, first option for MultipleChoice, "auto-approved" for Freeform
- **Console** (`console.rs:49-86`): `[?]` prefix, option display, freeform fallback, Y/N for YesNo, `>` prompt for Freeform
- **Callback** (`callback.rs:6-22`): delegates to `Box<dyn Fn(Question) -> Answer>`
- **Queue** (`queue.rs:9-28`): `Mutex<VecDeque<Answer>>`, returns SKIPPED when empty
- **Recording** (`recording.rs:8-39`): wraps inner interviewer, records `(Question, Answer)` pairs

### 47. Section 6.5 — Timeout Handling
**ALIGNED**
At `mod.rs:133-149`: uses `tokio::time::timeout`, returns `default_answer.unwrap_or_else(Answer::timeout)`. Tests at lines 269-296.

---

## Section 7: Validation and Linting

### 48. Section 7.1 — Diagnostic Model
**ALIGNED**
At `validation/mod.rs:9-25`: `Severity` (Error/Warning/Info), `Diagnostic` with `rule`, `severity`, `message`, `node_id`, `edge`, `fix` — all fields match.

### 49. Section 7.2 — Built-In Lint Rules
**ALIGNED** (14/14 rules implemented)
At `validation/rules.rs`:
| Rule | Severity | Location |
|------|----------|----------|
| `start_node` | ERROR | lines 30-69 |
| `terminal_node` | ERROR | lines 73-104 |
| `reachability` | ERROR | lines 108-155 (BFS) |
| `edge_target_exists` | ERROR | lines 159-201 |
| `start_no_incoming` | ERROR | lines 205-233 |
| `exit_no_outgoing` | ERROR | lines 237-272 |
| `condition_syntax` | ERROR | lines 276-316 |
| `stylesheet_syntax` | ERROR | lines 320-348 |
| `type_known` | WARNING | lines 352-392 |
| `fidelity_valid` | WARNING | lines 396-471 |
| `retry_target_exists` | WARNING | lines 475-548 |
| `goal_gate_has_retry` | WARNING | lines 552-586 |
| `prompt_on_llm_nodes` | WARNING | lines 590-624 |
| `freeform_edge_count` | ERROR | lines 628-663 |

**Minor:** `stylesheet_syntax` only checks brace balance, not full parse.

### 50. Section 7.3 — Validation API
**ALIGNED**
`validate(graph, extra_rules)` at line 35, `validate_or_raise(graph, extra_rules)` at line 52.

### 51. Section 7.4 — Custom Lint Rules
**ALIGNED**
`LintRule` trait at `mod.rs:28-31` with `name()` and `apply()`. Custom rules via `extra_rules` parameter.

---

## Section 8: Model Stylesheet

### 52. Section 8.1 — CSS-like Syntax
**ALIGNED**
`parse_stylesheet` at `stylesheet.rs:54-85` parses selector blocks with property declarations.

### 53. Section 8.2 — Selectors and Specificity
**ALIGNED** (minor extension)
Implementation at `stylesheet.rs:18-27` adds a `Shape` selector beyond spec:
| Selector | Specificity |
|----------|-------------|
| `*` (Universal) | 0 |
| Shape (bare word) | 1 |
| `.class` | 2 |
| `#id` | 3 |

Spec defines `*`=0, `.class`=1, `#id`=2. Relative ordering preserved; the `Shape` selector is an undocumented extension. Cascading behavior correct.

### 54. Section 8.3 — Application Order
**ALIGNED**
At `stylesheet.rs:190-238`: sorts by specificity, higher overwrites lower, explicit node attributes always override. Test at `stylesheet.rs:395-442` verifies spec section 8.6 example exactly.

### 55. Section 8.4 — Recognized Properties
**ALIGNED**
`STYLESHEET_PROPERTIES` at line 182: `["llm_model", "llm_provider", "reasoning_effort"]`. Exact match.

---

## Section 9: Transforms and Extensibility

### 56. Section 9.1 — Transform Trait
**ALIGNED**
At `transform.rs:5-7`: `fn apply(&self, graph: &mut Graph)`. In-place mutation vs spec's return-new-graph — functionally equivalent.

### 57. Section 9.2 — Built-In Transforms
**ALIGNED**
Three built-in transforms:
- **Variable Expansion:** `transform.rs:10-25` — expands `$goal` in prompts
- **Stylesheet Application:** `transform.rs:52-65` — applies `model_stylesheet`
- **Preamble:** `transform.rs:28-49` — prepends `[Context mode: {fidelity}]` for non-full fidelity
- **Minor:** Preamble applied at parse time, not execution time; cannot incorporate runtime fidelity changes from edges

### 58. Section 9.3 — Custom Transforms
**ALIGNED**
`PipelineBuilder::register_transform()` at `pipeline.rs:24-26`. Custom transforms run after built-in, in registration order. Integration test at `pipeline.rs:149-169`.

### 59. Section 9.4 — Event Stream
**ALIGNED**
All spec event types implemented at `event.rs:5-74`:
- Pipeline lifecycle: `PipelineStarted`, `PipelineCompleted`, `PipelineFailed`
- Stage lifecycle: `StageStarted`, `StageCompleted`, `StageFailed`, `StageRetrying`
- Parallel: `ParallelStarted`, `ParallelBranchStarted`, `ParallelBranchCompleted`, `ParallelCompleted`
- Human: `InterviewStarted`, `InterviewCompleted`, `InterviewTimeout`
- Checkpoint: `CheckpointSaved`

Observer pattern via `EventEmitter::on_event()` at line 106. Engine emits throughout execution.

### 60. Section 9.5 — Tool Call Hooks
**ALIGNED** (minor discrepancy)
Pre/post hooks at `codergen.rs:98-131`. `resolve_hook()` at lines 56-62 checks node-level then graph-level.
- **Minor:** Pre-hook non-zero returns `Outcome::fail()` (stronger than spec's "skip the tool call")

### 61. Section 9.6 — HTTP Server Mode
**N/A** — Spec says "Implementations may expose..." (optional). Not implemented.

---

## Section 10: Condition Expression Language

### 62. Section 10.1 — Grammar
**ALIGNED**
At `condition.rs`: `&&` conjunction (line 29), `!=` (lines 33-45), `=` (lines 46-58), bare key truthy (lines 59-72).

### 63. Section 10.2 — Semantics
**ALIGNED**
- Clauses AND-combined: `condition.rs:134` uses `.all()`
- `outcome` resolves to status string: line 88-89
- `preferred_label` resolves: lines 91-96
- `context.*` lookup with fallback: lines 98-105
- Missing keys = empty string: line 105
- Empty condition = true: lines 130-132

### 64. Section 10.3 — Variable Resolution
**ALIGNED**
`resolve_key()` at lines 87-110 follows spec pseudocode exactly: `outcome` → `preferred_label` → `context.` prefix with qualified/unqualified fallback → direct context lookup → empty string.

### 65. Section 10.4 — Examples
**ALIGNED**
Tests cover all spec examples: `outcome=success` (line 169), `context.tests_passed=true` (line 206), `preferred_label=Fix` (line 189).

### 66. Section 10.5 — Extended Operators
**ALIGNED**
Correctly NOT implemented per spec: "documented as potential extensions... Implementations should not add them."

---

## Section 11: Definition of Done

### 67. Section 11.1 — DOT Parsing
**ALIGNED**
Integration tests parse all 3 spec examples at `integration.rs:30-143`.

### 68. Section 11.2 — Validation and Linting
**ALIGNED**
14 lint rules, `validate_or_raise()` used in integration tests.

### 69. Section 11.3 — Execution Engine
**ALIGNED**
Start node resolution, handler dispatch, outcome recording, edge selection, loop execution, terminal stop — all verified in integration tests at `integration.rs:158-203`.

### 70. Section 11.4 — Goal Gate Enforcement
**ALIGNED**
Integration tests at `integration.rs:432-608`.

### 71. Section 11.5 — Retry Logic
**ALIGNED**
Integration test at `integration.rs:828-902`.

### 72. Section 11.6 — Node Handlers
**ALIGNED**
All handler types exist. Custom handler registration works.

### 73. Section 11.7 — State and Context
**ALIGNED**
Context updates, checkpoint save/resume, artifacts — verified at `integration.rs:978-1016`.

### 74. Section 11.8 — Human-in-the-Loop
**ALIGNED**
All interviewer implementations present. Integration test with QueueInterviewer at `integration.rs:376-381`.

### 75. Section 11.9 — Condition Expressions
**ALIGNED**
All operators and variable types tested at `condition.rs:160-323`.

### 76. Section 11.10 — Model Stylesheet
**ALIGNED**
Integration tests at `integration.rs:677-822` verify selectors, specificity, cascading.

### 77. Section 11.11 — Transforms
**ALIGNED**
Transform interface, variable expansion, custom transforms — all tested.

### 78. Section 11.12 — Cross-Feature Parity Matrix
**ALIGNED** (minor gap)
22 of 23 matrix items pass. **Minor:** No dedicated integration test for parallel fan-out/fan-in (handlers exist, no end-to-end test).

### 79. Section 11.13 — Integration Smoke Test
**ALIGNED**
`integration.rs:1040-1180` implements mock-backend smoke test matching spec pattern.

---

## Appendices

### 80. Appendix A — Complete Attribute Reference
**ALIGNED**
All graph, node, and edge attributes have corresponding accessors. Dotted keys (`tool_hooks.pre/post`, `stack.*`) supported via `lexer.rs:314-315`.

### 81. Appendix B — Shape-to-Handler-Type Mapping
**ALIGNED**
All 9 mappings tested at `graph/types.rs:413-429`.

### 82. Appendix C — Status File Contract
**GAP**
`auto_status=true` synthesis not implemented in engine (see item 23). `Outcome` struct at `outcome.rs:48-60` matches contract fields, but the engine never checks `auto_status`.

### 83. Appendix D — Error Categories
**ALIGNED** (minor gap)
`AttractorError` at `error.rs:4-25` has 7 variants: `Parse`, `Validation`, `Engine`, `Handler`, `Checkpoint`, `Stylesheet`, `Io`.
**Minor:** Spec defines 3 abstract categories (Retryable, Terminal, Pipeline). No explicit classification of which variants are retryable vs terminal; default `should_retry` retries all.

---

## Summary

| # | Section | Verdict |
|---|---------|---------|
| 1 | 1.1 Problem Statement | ALIGNED |
| 2 | 1.2 Why DOT Syntax | ALIGNED |
| 3 | 1.3 Design Principles | ALIGNED |
| 4 | 1.4 Layering / LLM Backends | ALIGNED |
| 5 | 2.1 Supported Subset | ALIGNED |
| 6 | 2.2 BNF Grammar | ALIGNED (minor: Direction not validated) |
| 7 | 2.3 Key Constraints | ALIGNED |
| 8 | 2.4 Value Types | ALIGNED |
| 9 | 2.5 Graph-Level Attributes | ALIGNED |
| 10 | 2.6 Node Attributes | ALIGNED |
| 11 | 2.7 Edge Attributes | ALIGNED |
| 12 | 2.8 Shape-to-Handler Mapping | ALIGNED |
| 13 | 2.9 Chained Edges | ALIGNED |
| 14 | 2.10-2.12 Subgraphs/Defaults/Class | ALIGNED |
| 15 | 3.1 Run Lifecycle | ALIGNED |
| 16 | 3.2 Core Execution Loop | ALIGNED |
| 17 | 3.3 Edge Selection Algorithm | ALIGNED |
| 18 | 3.4 Goal Gate Enforcement | ALIGNED |
| 19 | 3.5 Retry Logic | ALIGNED (minor: counters not persisted) |
| 20 | 3.6 Retry Policy / Backoff | ALIGNED (minor: should_retry too coarse) |
| 21 | 3.7 Failure Routing | ALIGNED |
| 22 | 3.8 Concurrency Model | ALIGNED |
| 23 | 3.9 Auto Status | **GAP** |
| 24 | 3.10 Timeout Enforcement | **GAP** |
| 25 | 4.1 Handler Trait | ALIGNED |
| 26 | 4.2 Handler Registry | ALIGNED |
| 27 | 4.3 Start Handler | ALIGNED |
| 28 | 4.4 Exit Handler | ALIGNED |
| 29 | 4.5 Codergen Handler | ALIGNED |
| 30 | 4.6 Wait Human Handler | ALIGNED |
| 31 | 4.7 Conditional Handler | ALIGNED |
| 32 | 4.8 Parallel Handler | ALIGNED |
| 33 | 4.9 Fan-In Handler | ALIGNED (minor: no score sort, all-fail case) |
| 34 | 4.10 Tool Handler | ALIGNED |
| 35 | 4.11 Manager Loop Handler | **GAP** (child_autostart missing) |
| 36 | 4.12 Custom Handlers | ALIGNED |
| 37 | 5.1 Context | ALIGNED |
| 38 | 5.2 Outcome | ALIGNED |
| 39 | 5.3 Checkpoint | ALIGNED (minor: retry counters, fidelity degradation on resume) |
| 40 | 5.4 Fidelity Modes | ALIGNED (minor: thread_id resolution incomplete) |
| 41 | 5.5 Artifact Store | ALIGNED |
| 42 | 5.6 Run Directory | ALIGNED (minor: manifest missing goal) |
| 43 | 6.1 Interviewer Interface | ALIGNED |
| 44 | 6.2 Question Model | ALIGNED |
| 45 | 6.3 Answer Model | ALIGNED |
| 46 | 6.4 Built-In Interviewers | ALIGNED |
| 47 | 6.5 Timeout Handling | ALIGNED |
| 48 | 7.1 Diagnostic Model | ALIGNED |
| 49 | 7.2 Built-In Lint Rules | ALIGNED (14/14) |
| 50 | 7.3 Validation API | ALIGNED |
| 51 | 7.4 Custom Lint Rules | ALIGNED |
| 52 | 8.1 CSS-like Syntax | ALIGNED |
| 53 | 8.2 Selectors/Specificity | ALIGNED (extra Shape selector) |
| 54 | 8.3 Application Order | ALIGNED |
| 55 | 8.4 Recognized Properties | ALIGNED |
| 56 | 9.1 Transform Trait | ALIGNED |
| 57 | 9.2 Built-In Transforms | ALIGNED |
| 58 | 9.3 Custom Transforms | ALIGNED |
| 59 | 9.4 Event Stream | ALIGNED |
| 60 | 9.5 Tool Call Hooks | ALIGNED (minor: pre-hook behavior) |
| 61 | 9.6 HTTP Server Mode | N/A (optional) |
| 62 | 10.1 Grammar | ALIGNED |
| 63 | 10.2 Semantics | ALIGNED |
| 64 | 10.3 Variable Resolution | ALIGNED |
| 65 | 10.4 Examples | ALIGNED |
| 66 | 10.5 Extended Operators | ALIGNED |
| 67 | 11.1 DOT Parsing | ALIGNED |
| 68 | 11.2 Validation | ALIGNED |
| 69 | 11.3 Execution Engine | ALIGNED |
| 70 | 11.4 Goal Gates | ALIGNED |
| 71 | 11.5 Retry Logic | ALIGNED |
| 72 | 11.6 Node Handlers | ALIGNED |
| 73 | 11.7 State/Context | ALIGNED |
| 74 | 11.8 Human-in-the-Loop | ALIGNED |
| 75 | 11.9 Conditions | ALIGNED |
| 76 | 11.10 Stylesheet | ALIGNED |
| 77 | 11.11 Transforms | ALIGNED |
| 78 | 11.12 Parity Matrix | ALIGNED (minor: no parallel integration test) |
| 79 | 11.13 Smoke Test | ALIGNED |
| 80 | Appendix A — Attributes | ALIGNED |
| 81 | Appendix B — Shape Mapping | ALIGNED |
| 82 | Appendix C — Status File | **GAP** (auto_status not enforced) |
| 83 | Appendix D — Error Categories | ALIGNED (minor: no retryable/terminal classification) |

---

## Totals

**79 ALIGNED / 3 GAP / 1 N/A** (+ 16 minor gaps within ALIGNED items)

### Hard Gaps (3)

1. **Auto Status (item 23/82):** `auto_status` accessor exists at `graph/types.rs:193-194` but engine never checks it. No auto-synthesis of SUCCESS when handler writes no status.

2. **Timeout Enforcement (item 24):** `timeout` attribute parsed as `Duration` but not enforced as a deadline around handler execution in the engine. Tool handler uses it for subprocess timeout, but no general enforcement.

3. **Manager Loop `child_autostart` (item 35):** `stack.child_autostart` / `start_child_pipeline` not implemented. The observe/steer/wait cycle exists but cannot auto-launch a child pipeline.

### Notable Minor Gaps (within ALIGNED items)

- Thread ID resolution: only step 1 of 5 implemented (node `thread_id`); missing edge, graph default, subgraph class, previous-node fallback
- Checkpoint resume: retry counters not restored; fidelity degradation (`full` → `summary:high`) not applied
- Retry policy: `should_retry` retries ALL errors; spec defines granular HTTP-status-based behavior
- Stylesheet: adds undocumented `Shape` selector (functional, shifts specificity values)
- Preamble transform: applied at parse time, not execution time
- No dedicated parallel fan-out/fan-in integration test
