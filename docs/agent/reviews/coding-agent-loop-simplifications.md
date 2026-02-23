# coding-agent-loop Simplification Proposals

## 1. Duplicated Mock ExecutionEnvironment Implementations

### 1.1 Duplicate full-trait mocks in tools.rs tests

**File:** `crates/coding-agent-loop/src/tools.rs`, lines 419-644
**Current:** Three separate structs (`ReadFileEnv`, `WriteFileEnv`, `EditFileEnv`, `ShellCapturingEnv`) each implement the full `ExecutionEnvironment` trait with 13 methods, where only 1-2 methods differ from the defaults. Each mock is ~60 lines of boilerplate.
**Simplification:** Extract a `DelegatingMockEnv` that wraps `MockExecutionEnvironment` and allows overriding specific methods via closures or composition. Alternatively, use the existing `MockExecutionEnvironment` with additional optional fields (like `files` for read, `written` capture for write, `captured_timeout` for shell). The `MockExecutionEnvironment` in `test_support.rs` already supports `files` and `exec_result` -- extending it with `Mutex<Option<(String,String)>>` for write captures would eliminate `WriteFileEnv`, `EditFileEnv`, and `ShellCapturingEnv` entirely.
**Why:** ~220 lines of near-identical boilerplate across four structs. Every time the `ExecutionEnvironment` trait changes, four mocks must be updated in addition to the ones in `test_support.rs` and `openai.rs`.

### 1.2 Duplicate MockFileEnv in openai.rs tests

**File:** `crates/coding-agent-loop/src/profiles/openai.rs`, lines 436-516
**Current:** `MockFileEnv` reimplements the full `ExecutionEnvironment` trait to support `Mutex<HashMap>` for write/delete operations in apply_patch tests.
**Simplification:** Consolidate into a single shared mock in `test_support.rs` that supports mutable file operations. The existing `MockExecutionEnvironment` already has a `files: HashMap` field -- wrapping it in `Mutex` (or making a `MutableMockEnv` variant) would replace `MockFileEnv`.
**Why:** Another ~80 lines of duplicated trait implementation. This is the same problem as 1.1 but in a different file.

---

## 2. Duplicated Profile Boilerplate

### 2.1 linux_env() helper duplicated across 4 test modules

**Files:**
- `crates/coding-agent-loop/src/profiles/mod.rs`, lines 96-103
- `crates/coding-agent-loop/src/profiles/anthropic.rs`, lines 164-170
- `crates/coding-agent-loop/src/profiles/gemini.rs`, lines 215-220
- `crates/coding-agent-loop/src/profiles/openai.rs`, lines 425-432

**Current:** Each test module defines an identical `linux_env()` function that creates a `MockExecutionEnvironment` with `working_dir: "/home/test"`, `platform_str: "linux"`, `os_version_str: "Linux 6.1.0"`.
**Simplification:** Add `MockExecutionEnvironment::linux()` as a named constructor in `test_support.rs`.
**Why:** Four identical copies of the same 6-line function. If the mock struct changes, all four must be updated.

### 2.2 Repetitive ProviderProfile implementations across three profiles

**Files:**
- `crates/coding-agent-loop/src/profiles/anthropic.rs`, lines 41-57
- `crates/coding-agent-loop/src/profiles/gemini.rs`, lines 41-56
- `crates/coding-agent-loop/src/profiles/openai.rs`, lines 43-57

**Current:** `id()`, `model()`, `tool_registry()`, and `tool_registry_mut()` have identical implementations in all three profile structs. The only differences are the string returned by `id()`.
**Simplification:** Introduce a `BaseProfile` struct containing the common `model: String` and `registry: ToolRegistry` fields, then each profile delegates to it. This could be done with a macro or simple struct composition. For example:

```rust
struct BaseProfile {
    id: &'static str,
    model: String,
    registry: ToolRegistry,
}
```

Each profile wraps `BaseProfile` and the four boilerplate methods delegate to it.
**Why:** Removes ~15 lines of identical code per profile (45 lines total) and makes it impossible for them to drift.

### 2.3 ParallelTestProfile vs TestProfile duplication

**File:** `crates/coding-agent-loop/src/test_support.rs`, lines 126-420
**Current:** `ParallelTestProfile` and `TestProfile` are nearly identical `ProviderProfile` implementations. The only difference is `supports_parallel_tool_calls` (false vs true) and an optional `context_window` field.
**Simplification:** Merge into a single `TestProfile` with configurable fields:

```rust
pub(crate) struct TestProfile {
    pub registry: ToolRegistry,
    pub parallel_tool_calls: bool,
    pub context_window: usize,
}
```

The `with_tools` constructor defaults `parallel_tool_calls` to `false` and `context_window` to `200_000`. A `.with_parallel()` builder method or a `TestProfileBuilder` enables the parallel variant.
**Why:** Eliminates ~60 lines of duplicated trait implementation and makes the test intention clearer.

---

## 3. Redundant or Unused Code

### 3.1 History::new() is redundant with Default derive

**File:** `crates/coding-agent-loop/src/history.rs`, lines 10-12
**Current:** `History::new()` manually creates `Self { turns: Vec::new() }`, and `#[derive(Default)]` is on the struct.
**Simplification:** Remove the manual `new()` method entirely and use `History::default()` everywhere, or keep `new()` but implement it as `Self::default()`. Currently both exist and do the same thing.
**Why:** Two ways to do the same thing is confusing. Pick one.

### 3.2 EventEmitter::new() is redundant with Default impl

**File:** `crates/coding-agent-loop/src/event.rs`, lines 12-15 and 34-38
**Current:** `new()` and `default()` are both defined, `default()` just calls `new()`.
**Simplification:** This is a standard Rust pattern and is fine, but `#[must_use]` on `new()` but not on `Default::default()` is inconsistent. Consider removing the manual `Default` impl and adding `#[must_use]` consistently, or just keep one constructor.
**Why:** Minor, but reduces cognitive load.

---

## 4. Control Flow Simplifications

### 4.1 Simplify followup loop in process_input

**File:** `crates/coding-agent-loop/src/session.rs`, lines 193-208
**Current:**
```rust
loop {
    self.run_single_input(&current_input).await?;
    let next_followup = self.followup_queue.lock()
        .expect("followup queue lock poisoned")
        .pop_front();
    match next_followup {
        Some(followup) => { current_input = followup; }
        None => break,
    }
}
```
**Simplification:** Use `while let`:
```rust
self.run_single_input(&current_input).await?;
while let Some(followup) = self.followup_queue.lock()
    .expect("followup queue lock poisoned")
    .pop_front()
{
    self.run_single_input(&followup).await?;
}
```
**Why:** Eliminates the mutable `current_input` variable and the `loop`/`match`/`break` pattern. The intent is clearer: process the initial input, then process followups until the queue is empty.

### 4.2 Simplify SubAgentManager::spawn success path

**File:** `crates/coding-agent-loop/src/subagent.rs`, lines 67-87
**Current:**
```rust
let task = tokio::spawn(async move {
    let result = session.process_input(&task_prompt).await;
    let turns = session.history().turns();
    let turns_used = turns.len();
    let last_text = turns.iter().rev().find_map(|t| {
        if let Turn::Assistant { content, .. } = t {
            Some(content.clone())
        } else {
            None
        }
    });
    let success = result.is_ok();
    if let Err(e) = result {
        return Err(e);
    }
    Ok(SubAgentResult {
        output: last_text.unwrap_or_default(),
        success,
        turns_used,
    })
});
```
**Simplification:** The `success` variable is computed from `result.is_ok()`, then `result` is checked for `Err` immediately after. Since `success` is always `true` when reaching the `Ok` path:
```rust
let task = tokio::spawn(async move {
    session.process_input(&task_prompt).await?;
    let turns = session.history().turns();
    let last_text = turns.iter().rev().find_map(|t| match t {
        Turn::Assistant { content, .. } => Some(content.clone()),
        _ => None,
    });
    Ok(SubAgentResult {
        output: last_text.unwrap_or_default(),
        success: true,
        turns_used: turns.len(),
    })
});
```
**Why:** Eliminates the unnecessary `success` variable (always `true` on the Ok path) and the redundant `if let Err(e) = result { return Err(e) }` pattern which is just `result?`.

### 4.3 validate_tool_args empty schema check is overly complex

**File:** `crates/coding-agent-loop/src/session.rs`, lines 654-658
**Current:**
```rust
if schema.is_null()
    || (schema.is_object() && schema.as_object().map_or(true, |o| o.is_empty()))
{
    return Ok(());
}
```
**Simplification:** The `map_or(true, |o| o.is_empty())` is confusing because `as_object()` returns `None` when `is_object()` is false, but we already checked `is_object()`. So the `map_or(true, ...)` default of `true` is dead code. Simplify to:
```rust
if schema.is_null() {
    return Ok(());
}
if let Some(obj) = schema.as_object() {
    if obj.is_empty() {
        return Ok(());
    }
}
```
**Why:** The original combines null check and empty-object check with boolean operators in a way that requires careful reading. The separated version is immediately clear.

---

## 5. Structural / Architectural Simplifications

### 5.1 extract_signatures_from_assistant should use if-let instead of match

**File:** `crates/coding-agent-loop/src/loop_detection.rs`, lines 14-22
**Current:**
```rust
fn extract_signatures_from_assistant(turn: &Turn) -> Vec<u64> {
    match turn {
        Turn::Assistant { tool_calls, .. } => tool_calls
            .iter()
            .map(|tc| tool_call_signature(&tc.name, &tc.arguments))
            .collect(),
        _ => vec![],
    }
}
```
**Simplification:** This function is only called in one place (line 35), where the result is immediately checked with `if !sigs.is_empty()`. The function could be inlined, but even if kept, consider using `if let`:
```rust
fn extract_signatures_from_assistant(turn: &Turn) -> Vec<u64> {
    let Turn::Assistant { tool_calls, .. } = turn else {
        return vec![];
    };
    tool_calls
        .iter()
        .map(|tc| tool_call_signature(&tc.name, &tc.arguments))
        .collect()
}
```
**Why:** The `let-else` pattern makes the happy path less indented and immediately shows the function's purpose.

### 5.2 build_request constructs system prompt on every call

**File:** `crates/coding-agent-loop/src/session.rs`, lines 371-403
**Current:** `build_request()` calls `build_system_prompt()` every iteration of the tool-call loop (called from `run_single_input` inside the `loop` at line 237). The system prompt, project docs, and environment context do not change during a single input processing cycle.
**Simplification:** Compute the system prompt once at the start of `run_single_input` and pass it into `build_request`, or cache it as a field that's rebuilt only when `initialize()` or `process_input()` is called.
**Why:** Avoids redundant string allocation and concatenation on every LLM round-trip. For sessions with many tool rounds, this is significant wasted work.

### 5.3 estimate_token_count also rebuilds the system prompt

**File:** `crates/coding-agent-loop/src/session.rs`, lines 514-553
**Current:** `estimate_token_count()` calls `build_system_prompt()` again to get its length, duplicating the work already done in `build_request()` on the same iteration.
**Simplification:** If the system prompt is cached per proposal 5.2, `estimate_token_count` can read from the cache. Alternatively, pass the already-built system prompt length to `check_context_usage`.
**Why:** Double construction of the system prompt per LLM call is wasteful.

### 5.4 ProviderProfile trait returns owned Strings unnecessarily

**File:** `crates/coding-agent-loop/src/provider_profile.rs`, lines 20-21
**Current:** `fn id(&self) -> String` and `fn model(&self) -> String` return owned `String` values. Every call allocates.
**Simplification:** Return `&str` instead:
```rust
fn id(&self) -> &str;
fn model(&self) -> &str;
```
All implementations store the model as a `String` field and the id as a string literal, so returning `&str` is straightforward.
**Why:** Eliminates unnecessary heap allocation on every call. These methods are called frequently (every `build_request` call).

---

## 6. Naming and Clarity

### 6.1 EnvContext fields lack consistent naming

**File:** `crates/coding-agent-loop/src/profiles/mod.rs`, lines 13-21
**Current:** The struct has fields `date`, `model_name`, `knowledge_cutoff` alongside `git_branch`, `is_git_repo`. The non-git fields use varying naming conventions -- `date` is vague (what date?), `model_name` is redundant (just `model` would match `ProviderProfile::model()`).
**Simplification:** Rename `date` to `today` or `current_date`, and `model_name` to `model` for consistency with the trait method name.
**Why:** Clearer intent and consistent naming.

### 6.2 Turn::Steering and Turn::System are semantically close

**File:** `crates/coding-agent-loop/src/types.rs`, lines 5-29
**Current:** `Turn::System` and `Turn::Steering` both represent injected content. `System` maps to `Role::System` in the LLM message, while `Steering` maps to `Role::User`.
**Simplification:** No code change needed, but adding a brief doc comment to each variant clarifying the distinction would help. Currently there's no documentation explaining when to use which.
**Why:** A reader must trace through `convert_to_messages()` to understand the difference.

---

## 7. Tool Construction Boilerplate

### 7.1 Repeated parameter extraction pattern in tool executors

**Files:** `crates/coding-agent-loop/src/tools.rs` (lines 26-28, 61-66, 94-106, 168-175, 216-237, 263-269) and `crates/coding-agent-loop/src/subagent.rs` (lines 189-193, 237-244, 275-278, 312-315)

**Current:** Every tool executor manually extracts parameters with the same pattern:
```rust
let param = args.get("param")
    .and_then(|v| v.as_str())
    .ok_or_else(|| "Missing required parameter: param".to_string())?;
```
This pattern repeats ~15 times across the codebase with slight variations.
**Simplification:** Introduce a small helper:
```rust
fn required_str<'a>(args: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing required parameter: {key}"))
}
```
**Why:** Reduces boilerplate and ensures consistent error messages across all tools.

---

## 8. Dead or Near-Dead Code

### 8.1 build_env_context_block (no-context variant) has limited use

**File:** `crates/coding-agent-loop/src/profiles/mod.rs`, lines 50-53
**Current:** `build_env_context_block` wraps `build_env_context_block_with` with a default `EnvContext`. It's only used in one test.
**Simplification:** Inline the default at the one test call site. Or keep it as a convenience but make it `#[cfg(test)]`.
**Why:** Public API surface should be intentional. If this is only for tests, mark it as such.

### 8.2 SubAgent::id() and SubAgent::depth() are only used in tests

**File:** `crates/coding-agent-loop/src/subagent.rs`, lines 28-35
**Current:** `SubAgent` has `id()` and `depth()` accessor methods.
**Simplification:** Verify these are used outside tests. If they are only used in the test at line 348 (`manager.get(&agent_id).unwrap().depth()`), consider whether the `get()` method on the manager (and these accessors) serve a real purpose, or if they exist only to test internals.
**Why:** Exposing internal state for testing purposes adds API surface that must be maintained.

### 8.3 Unused import: std::sync::Arc in profiles/openai.rs

**File:** `crates/coding-agent-loop/src/profiles/openai.rs`, line 9
**Current:** `use std::sync::Arc;` is imported at the module level. It's used only in `make_apply_patch_tool()` for the executor closure.
**Simplification:** This is fine for production code. Just noting it's not used in the profile implementation itself, only in the private tool factory.
**Why:** Minor observation, no action needed.

---

## 9. Error Handling

### 9.1 Inconsistent error types: String vs AgentError

**Files:** Throughout the crate
**Current:** The `ExecutionEnvironment` trait uses `Result<T, String>`, tool executors return `Result<String, String>`, while `Session` methods return `Result<(), AgentError>`. The `SubAgentManager` also uses `Result<T, String>`.
**Simplification:** Consider using `AgentError` (or a dedicated `ToolError`) throughout instead of raw `String` errors. At minimum, `SubAgentManager` methods that return user-facing errors should use a typed error.
**Why:** `String` errors lose the ability to match on error variants, making programmatic error handling impossible. This is a larger refactor but would significantly improve the API.

### 9.2 Lock poisoning panics could be handled

**Files:** `crates/coding-agent-loop/src/session.rs` (lines 143, 149, 199, 354) and `crates/coding-agent-loop/src/subagent.rs` (line 112)
**Current:** `.expect("...lock poisoned")` is used on every mutex lock.
**Simplification:** This is actually fine for most use cases -- a poisoned lock indicates a panic occurred while the lock was held, which is a serious bug. No change recommended, but documenting the deliberate choice would help.
**Why:** No action needed. This is the standard Rust approach.

---

## 10. Test Organization

### 10.1 ProviderTestProfile in provider_profile.rs duplicates TestProfile

**File:** `crates/coding-agent-loop/src/provider_profile.rs`, lines 86-139
**Current:** A `ProviderTestProfile` struct is defined with a custom `build_system_prompt` that includes platform info. It exists alongside `TestProfile` in `test_support.rs`.
**Simplification:** If the test-specific behavior (including platform in prompt) matters, add a flag to `TestProfile` to enable it. If not, use `TestProfile` directly.
**Why:** The comment on line 85 says "uses distinct id/model and a custom build_system_prompt", but examining the tests, the custom prompt is only checked for containing "linux" and "1" (docs count). These tests could use the shared `TestProfile` with a more flexible assertion.

### 10.2 Session tests define CapturingProvider inline

**File:** `crates/coding-agent-loop/src/session.rs`, lines 1225-1254 and 1411-1432
**Current:** Two separate inline `CapturingProvider` structs are defined within test functions in the same module. Both capture request data for assertion.
**Simplification:** Extract a single `CapturingLlmProvider` into `test_support.rs` that captures the full request, then tests can assert on whichever field they need. The first one (line 1225) captures `reasoning_effort`, the second (line 1411) captures messages. A single mock that captures the full `Request` would serve both.
**Why:** Two near-identical mock providers in the same test module is unnecessary duplication.

---

## Summary

| Category | Count | Estimated Lines Saved |
|----------|-------|-----------------------|
| Duplicated mock implementations | 3 | ~350 |
| Duplicated profile boilerplate | 3 | ~120 |
| Redundant code | 2 | ~15 |
| Control flow simplifications | 3 | ~20 |
| Structural improvements | 4 | ~50 (plus perf gains) |
| Naming and clarity | 2 | 0 (documentation) |
| Tool construction boilerplate | 1 | ~30 |
| Dead or near-dead code | 3 | ~20 |
| Error handling | 2 | 0 (design decision) |
| Test organization | 2 | ~80 |
| **Total** | **25** | **~685** |

The highest-impact simplifications are:
1. **Consolidating mock ExecutionEnvironment implementations** (proposals 1.1, 1.2) -- eliminates the most duplicated code
2. **Merging TestProfile and ParallelTestProfile** (proposal 2.3) -- reduces test infrastructure
3. **Caching system prompt per input cycle** (proposals 5.2, 5.3) -- both a clarity and performance win
4. **Returning &str from ProviderProfile::id() and ::model()** (proposal 5.4) -- cleaner API
5. **Helper for parameter extraction** (proposal 7.1) -- reduces the most pervasive boilerplate pattern
