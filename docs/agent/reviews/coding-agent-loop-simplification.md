# coding-agent-loop Simplification Analysis

Date: 2026-02-20

## Executive Summary

The `coding-agent-loop` crate is approximately 4,800 lines of production code and tests across 19 source files. The architecture is generally sound, but there are significant opportunities to reduce complexity, eliminate duplication, and improve maintainability. The most impactful findings center on massive test mock duplication, duplicated tool execution logic in `session.rs`, and the `ProviderProfile` trait being too wide.

---

## HIGH Severity Findings

### 1. Massive Mock `ExecutionEnvironment` Duplication Across Tests

**What:** The `ExecutionEnvironment` trait has 12 methods, and a full mock implementation is copy-pasted into nearly every test module. I count at least **11 separate mock implementations** of `ExecutionEnvironment` spread across:

- `execution_env.rs` (`MockEnv`)
- `tool_registry.rs` (`DummyEnv`)
- `tools.rs` (`ReadFileEnv`, `WriteFileEnv`, `EditFileEnv`, `ShellEnv`, `ShellCapturingEnv`, `GrepEnv`, `GlobEnv`)
- `provider_profile.rs` (`TestEnv`)
- `project_docs.rs` (`DocEnv`)
- `profiles/mod.rs` (`TestEnv`)
- `profiles/anthropic.rs` (`TestEnv`)
- `profiles/gemini.rs` (`TestEnv`)
- `profiles/openai.rs` (`TestEnv`, `MockFileEnv`)
- `subagent.rs` (`MemoryExecutionEnvironment`)
- `session.rs` (`MemoryExecutionEnvironment`)

Each one is 30-60 lines of boilerplate implementing every trait method. Most implementations are identical stubs returning empty/default values, with only 1-2 methods customized per mock.

**Where:** Every file with `#[cfg(test)]` modules.

**Simplification:** Create a single `MockExecutionEnvironment` in a shared test utility module (e.g., `src/test_support.rs` behind `#[cfg(test)]`) that provides sensible defaults. Specific tests can then wrap or override individual methods using composition or builder patterns. This would eliminate approximately **500-700 lines** of duplicated test code.

```rust
// src/test_support.rs
#[cfg(test)]
pub struct MockExecutionEnvironment {
    pub files: std::collections::HashMap<String, String>,
    pub exec_result: Option<ExecResult>,
    pub grep_results: Vec<String>,
    pub glob_results: Vec<String>,
    // ...
}
```

**Impact:** HIGH -- this is the single largest source of unnecessary code in the crate. It also makes adding new methods to `ExecutionEnvironment` extremely painful since every mock must be updated.

---

### 2. Duplicated Tool Execution Logic Between Sequential and Parallel Paths

**What:** `session.rs` contains two nearly identical implementations of tool execution:

1. `execute_single_tool` + `emit_execute_and_truncate` (used by the sequential path)
2. The inline closure in `execute_tool_calls_parallel` (lines 507-613)

Both paths:
- Emit `ToolCallStart` events
- Look up the tool in the registry
- Validate arguments against the schema
- Execute the tool
- Handle success/error into `ToolResult`
- Emit `ToolCallEnd` events with output data
- Truncate the output for history

The parallel path duplicates all of this logic inside a closure, including identical `ToolResult` construction, identical event emission, and identical truncation.

**Where:** `/crates/coding-agent-loop/src/session.rs`, lines 425-668.

**Simplification:** Extract a shared `execute_one_tool` function that takes the necessary context (emitter, registry, env, config, session_id) and returns the truncated `ToolResult`. Both the sequential and parallel paths should call this same function. The parallel path simply runs multiple instances concurrently with `join_all`.

This would eliminate approximately **80-100 lines** of duplicated logic and ensure bug fixes apply to both paths.

**Impact:** HIGH -- duplicated business logic is a correctness risk; fixing a bug in one path but not the other is easy.

---

### 3. `ProviderProfile` Trait Is Too Wide (14 Methods)

**What:** The `ProviderProfile` trait requires implementing 14 methods:

```rust
pub trait ProviderProfile: Send + Sync {
    fn id(&self) -> String;
    fn model(&self) -> String;
    fn tool_registry(&self) -> &ToolRegistry;
    fn tool_registry_mut(&mut self) -> &mut ToolRegistry;
    fn build_system_prompt(...) -> String;
    fn tools(&self) -> Vec<ToolDefinition>;
    fn provider_options(&self) -> Option<serde_json::Value>;
    fn supports_reasoning(&self) -> bool;
    fn supports_streaming(&self) -> bool;
    fn supports_parallel_tool_calls(&self) -> bool;
    fn context_window_size(&self) -> usize;
    fn knowledge_cutoff(&self) -> &str;
}
```

Several of these are pure data fields that don't need virtual dispatch. The `tools()` method is always just `self.registry.definitions()`. The `tool_registry()` and `tool_registry_mut()` methods exist only to allow external registration of subagent tools. This forces every test to implement all 14 methods even when only 1-2 matter.

**Where:** `/crates/coding-agent-loop/src/provider_profile.rs`

**Simplification:** Consider replacing the trait with a struct that holds data fields plus a closure/trait for the only truly polymorphic behavior (`build_system_prompt`). Alternatively, add default implementations where possible (e.g., `fn tools(&self) -> Vec<ToolDefinition> { self.tool_registry().definitions() }`). At minimum, `tools()` should have a default implementation since it's identical in all 3 profiles and every test profile.

The `supports_*` methods and `context_window_size` could be a `ProfileCapabilities` struct to reduce the trait surface.

**Impact:** HIGH -- affects every test file and every new profile implementation.

---

## MEDIUM Severity Findings

### 4. `register_subagent_tools` Is Copy-Pasted Across All Three Profiles

**What:** The `register_subagent_tools` method is identical in `AnthropicProfile`, `GeminiProfile`, and `OpenAiProfile`:

```rust
pub fn register_subagent_tools(
    &mut self,
    manager: Arc<tokio::sync::Mutex<SubAgentManager>>,
    session_factory: SessionFactory,
    current_depth: usize,
) {
    self.registry.register(make_spawn_agent_tool(manager.clone(), session_factory, current_depth));
    self.registry.register(make_send_input_tool(manager.clone()));
    self.registry.register(make_wait_tool(manager.clone()));
    self.registry.register(make_close_agent_tool(manager));
}
```

**Where:** `profiles/anthropic.rs:45-60`, `profiles/gemini.rs:45-60`, `profiles/openai.rs:45-60`

**Simplification:** Move this to a free function or a method on `ToolRegistry`:

```rust
pub fn register_subagent_tools(
    registry: &mut ToolRegistry,
    manager: Arc<tokio::sync::Mutex<SubAgentManager>>,
    session_factory: SessionFactory,
    current_depth: usize,
) { ... }
```

Or add it as a default method on `ProviderProfile` since the trait already has `tool_registry_mut()`.

**Impact:** MEDIUM -- 3x duplication of 8 lines each. Easy to drift.

---

### 5. `build_system_prompt` Duplicated Structure Across Profiles

**What:** All three profiles' `build_system_prompt` methods share identical preamble and postamble logic:

```rust
let env_block = build_env_context_block_with(env, env_context);
let docs_section = if project_docs.is_empty() {
    String::new()
} else {
    format!("\n\n{}", project_docs.join("\n\n"))
};
let user_section = match user_instructions {
    Some(instructions) => format!("\n\n# User Instructions\n{instructions}"),
    None => String::new(),
};
```

This identical block appears in `anthropic.rs:87-96`, `gemini.rs:87-96`, and `openai.rs:87-96`. Only the core prompt text differs.

**Where:** All three profile files.

**Simplification:** Extract a helper that takes the core prompt as a parameter:

```rust
fn assemble_system_prompt(
    core_prompt: &str,
    env: &dyn ExecutionEnvironment,
    env_context: &EnvContext,
    project_docs: &[String],
    user_instructions: Option<&str>,
) -> String { ... }
```

Each profile would then only need to provide its unique prompt text.

**Impact:** MEDIUM -- reduces ~15 lines per profile, more importantly makes the structure consistent.

---

### 6. `SessionEvent.data` Uses `HashMap<String, serde_json::Value>` Instead of Typed Variants

**What:** Every event emitted throughout the codebase constructs a `HashMap<String, serde_json::Value>` manually:

```rust
let mut data = HashMap::new();
data.insert("tool_name".to_string(), serde_json::json!(&tc.name));
data.insert("tool_call_id".to_string(), serde_json::json!(&tc.id));
```

This pattern is repeated 15+ times across `session.rs`. The keys are stringly-typed and there's no compile-time guarantee about what data each event kind carries.

**Where:** `/crates/coding-agent-loop/src/session.rs` (throughout), `types.rs`

**Simplification:** Use typed event data enums:

```rust
pub enum EventData {
    Empty,
    ToolCall { tool_name: String, tool_call_id: String },
    ToolCallEnd { tool_name: String, tool_call_id: String, output: serde_json::Value, is_error: bool },
    Error { error: String },
    ContextWarning { estimated_tokens: usize, context_window_size: usize, usage_percent: usize },
}
```

This removes all the `HashMap::new()` / `.insert()` boilerplate and provides type safety.

**Impact:** MEDIUM -- affects readability and correctness of event handling code.

---

### 7. `tools.rs` Exports `make_read_many_files_tool`, `make_list_dir_tool`, `make_web_search_tool`, `make_web_fetch_tool` But They Are Not Re-exported from `lib.rs`

**What:** `lib.rs` only re-exports:
```rust
pub use tools::{
    make_edit_file_tool, make_glob_tool, make_grep_tool, make_read_file_tool, make_shell_tool,
    make_shell_tool_with_config, make_write_file_tool,
};
```

But `tools.rs` also defines `make_read_many_files_tool`, `make_list_dir_tool`, `make_web_search_tool`, and `make_web_fetch_tool`. These are used internally by profiles (Gemini uses all of them, OpenAI uses `apply_patch`) but are not available to external consumers.

**Where:** `/crates/coding-agent-loop/src/lib.rs:31-34`, `/crates/coding-agent-loop/src/tools.rs`

**Simplification:** Either re-export all tools from `lib.rs` for consistency, or make the non-exported ones `pub(crate)` to clarify they're internal. The current state is ambiguous -- they're `pub` in `tools.rs` but not re-exported, suggesting an oversight.

**Impact:** MEDIUM -- confusing public API surface.

---

### 8. `TestProfile` / `MockLlmProvider` Duplicated Between `session.rs` and `subagent.rs`

**What:** Both `session.rs` and `subagent.rs` define their own:
- `MockLlmProvider` (identical implementation)
- `TestProfile` (identical implementation)
- `MemoryExecutionEnvironment` (nearly identical)
- `text_response` helper (identical)
- `make_client` helper (identical)
- `make_session` helper (identical)

**Where:** `session.rs` tests (lines 785-1135) and `subagent.rs` tests (lines 340-536).

**Simplification:** Extract these into a shared test support module. This would save approximately **200 lines** of duplicated test infrastructure.

**Impact:** MEDIUM -- significant duplication that makes maintenance harder.

---

### 9. `Io(String)` Error Variant Is Never Constructed

**What:** `AgentError::Io(String)` is defined and tested but never actually used anywhere in the production code. No code path constructs this variant.

**Where:** `/crates/coding-agent-loop/src/error.rs:18-19`

**Simplification:** Remove the variant (and its test) if it's truly unused. If it's intended for future use, add a `#[allow(dead_code)]` with a comment explaining when it will be needed.

**Impact:** MEDIUM -- dead code.

---

### 10. `History::new()` and `Default` Redundancy

**What:** `History` derives `Default` and also has a `new()` method that does the same thing. Both `new()` and `default()` return `Self { turns: Vec::new() }`.

**Where:** `/crates/coding-agent-loop/src/history.rs:4-11`

**Simplification:** Remove the manual `new()` and use `Default::default()` everywhere, or keep `new()` and remove the `Default` derive. The codebase uses `History::new()` everywhere, so keeping `new()` is fine, but having both is unnecessary. The `#[derive(Default)]` could be kept for flexibility since it's zero-cost.

**Impact:** LOW (but worth noting for consistency).

---

## LOW Severity Findings

### 11. `count_turns` Method Is Just `len()` by Another Name

**What:** `History::count_turns()` simply returns `self.turns.len()`. The name `count_turns` doesn't add semantic value over `len()` given the method already returns `&[Turn]` via `turns()`.

**Where:** `/crates/coding-agent-loop/src/history.rs:22-24`

**Simplification:** Replace `count_turns()` calls with `turns().len()` and remove the method, or rename to `len()` to follow Rust convention.

**Impact:** LOW.

---

### 12. `build_request` Calls `self.provider_profile.tools()` Twice

**What:** In `session.rs` `build_request()`:
```rust
let tools = self.provider_profile.tools();
// ...
tools: if tools.is_empty() { None } else { Some(tools) },
tool_choice: if self.provider_profile.tools().is_empty() {  // <-- second call
    None
} else {
    Some(ToolChoice::Auto)
},
```

The second `self.provider_profile.tools()` call re-collects all tool definitions from the registry when it could just reuse the `tools` variable.

**Where:** `/crates/coding-agent-loop/src/session.rs:402-413`

**Simplification:**
```rust
let tools = self.provider_profile.tools();
let has_tools = !tools.is_empty();
// ...
tools: if has_tools { Some(tools) } else { None },
tool_choice: if has_tools { Some(ToolChoice::Auto) } else { None },
```

**Impact:** LOW -- minor inefficiency and readability issue.

---

### 13. `EnvContext` Fields `git_status_short` and `git_recent_commits` Are Populated But Never Used

**What:** `Session::build_env_context()` populates `git_status_short` and `git_recent_commits` from git commands, but `build_env_context_block_with()` never reads these fields. They are stored in the `EnvContext` struct but have no effect on the system prompt or any other behavior.

**Where:** `/crates/coding-agent-loop/src/session.rs:97-119`, `/crates/coding-agent-loop/src/profiles/mod.rs:29-55`

**Simplification:** Either use these fields in the environment context block (which seems to be the intent), or remove them and the git commands that populate them. Currently they cause two unnecessary shell invocations on every session initialization.

**Impact:** LOW -- dead code causing unnecessary I/O.

---

### 14. `truncation.rs` Rebuilds Default Limit HashMaps on Every Call

**What:** `truncate_tool_output` calls `default_char_limits()`, `default_line_limits()`, and `default_truncation_modes()` which each allocate and populate a new `HashMap` on every invocation.

**Where:** `/crates/coding-agent-loop/src/truncation.rs:87-121`

**Simplification:** Use `LazyLock` (stable in Rust 1.80+) or `const` arrays with a lookup function to avoid repeated allocation:

```rust
static DEFAULT_CHAR_LIMITS: LazyLock<HashMap<&str, usize>> = LazyLock::new(|| {
    // ...
});
```

Alternatively, replace the `HashMap` lookups with simple match statements since the key sets are small and fixed.

**Impact:** LOW -- minor allocation overhead per tool call, but tool calls are not in a hot path.

---

### 15. `GrepOptions` Uses `grep` CLI Fallback That Will Always Succeed (Hiding `rg` Not Found)

**What:** In `local_env.rs`, the `grep` method checks if `rg --version` succeeds. But `std::process::Command::new("rg").arg("--version").status().is_ok()` returns `Ok` as long as the process was *launched*, not necessarily that it succeeded. The `.is_ok()` check is on the `Result` from `status()`, not on the exit code.

**Where:** `/crates/coding-agent-loop/src/local_env.rs:246-251`

**Simplification:** Check the exit code:
```rust
let use_rg = std::process::Command::new("rg")
    .arg("--version")
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status()
    .map(|s| s.success())
    .unwrap_or(false);
```

**Impact:** LOW -- subtle correctness issue on systems where `rg` exists but returns an error.

---

### 16. `glob` Implementation Uses Shell Globbing via `ls -d` Which Is Fragile

**What:** The `glob` method in `local_env.rs` uses `sh -c "ls -d {pattern} 2>/dev/null"` to expand glob patterns. This is fragile because:
- Filenames with spaces or special characters will break
- The pattern is not shell-escaped
- `ls -d` behaves differently across platforms

**Where:** `/crates/coding-agent-loop/src/local_env.rs:300-333`

**Simplification:** Use the `glob` crate (a Rust-native glob implementation) instead of shelling out. This would be more reliable, cross-platform, and avoid shell injection concerns.

**Impact:** LOW for now (this is local-only), but worth addressing before any security-sensitive use.

---

### 17. Types Tests Are Overly Trivial

**What:** `types.rs` contains tests that merely construct enum variants and check that `PartialEq` works:

```rust
fn session_state_equality() {
    assert_eq!(SessionState::Idle, SessionState::Idle);
    assert_ne!(SessionState::Idle, SessionState::Closed);
}
```

These test the `#[derive(PartialEq)]` macro, which is guaranteed by the compiler.

**Where:** `/crates/coding-agent-loop/src/types.rs:67-184`

**Simplification:** Remove these tests. They add ~80 lines of code that test derived functionality and provide no value. The construction tests for `Turn` variants are slightly more useful as documentation but still marginal.

**Impact:** LOW -- no correctness value, just noise.

---

### 18. `apply_patch` Delete Operation Writes Empty String Instead of Deleting

**What:** `PatchOperation::Delete` is handled by writing an empty string to the file:

```rust
PatchOperation::Delete { path } => {
    env.write_file(path, "").await?;
    results.push(format!("Deleted file: {path}"));
}
```

This leaves a zero-byte file on disk rather than actually deleting it.

**Where:** `/crates/coding-agent-loop/src/profiles/openai.rs:359-362`

**Simplification:** Add a `delete_file` method to `ExecutionEnvironment`, or use `exec_command("rm ...")`. Writing empty content and calling it "deleted" is misleading.

**Impact:** LOW -- the current behavior may be intentional to avoid adding a `delete_file` method to the trait, but it's semantically wrong.

---

## Structural Observations

### File Organization

The module structure is reasonable. A few observations:

1. **`profiles/openai.rs` contains the entire v4a patch parser** (~200 lines). This is OpenAI-specific tooling that could be its own module (`src/patch_v4a.rs`) for clarity, since it's a self-contained parser/applier.

2. **`tools.rs` and `subagent.rs` both define tool factories** (functions that return `RegisteredTool`). The tools in `tools.rs` are "standard" tools while `subagent.rs` has subagent-specific tools. This split makes sense but the non-standard tools (`make_list_dir_tool`, `make_read_many_files_tool`, etc.) are only used by specific profiles and could be co-located with those profiles.

3. **`provider_profile.rs` and `profiles/mod.rs`** -- the trait is in one file and the `EnvContext` struct + `build_env_context_block` functions are in another. These are tightly coupled and could be consolidated.

### Approximate Line Count Savings

| Finding | Estimated Lines Saved |
|---------|----------------------|
| #1 Shared test mock | 500-700 |
| #2 Deduplicate tool execution | 80-100 |
| #4 Shared subagent registration | 20 |
| #5 Shared prompt assembly | 40 |
| #8 Shared test infrastructure | 200 |
| #9 Remove dead Io variant | 10 |
| #17 Remove trivial tests | 80 |
| **Total** | **~930-1150 lines** |

This represents roughly 20-25% of the crate's total size, with the vast majority coming from test deduplication.

---

## Recommended Priority Order

1. **Shared test mock for `ExecutionEnvironment`** (#1, #8) -- highest impact, eliminates the most duplication
2. **Deduplicate tool execution in session.rs** (#2) -- correctness risk
3. **Extract shared prompt assembly** (#5) + **shared subagent registration** (#4)
4. **Narrow the `ProviderProfile` trait** (#3) -- architectural improvement
5. **Fix unused `EnvContext` fields** (#13) -- removes unnecessary I/O
6. **Type the event data** (#6) -- readability improvement
7. **Clean up minor issues** (#9, #11, #12, #14, #15, #17)
