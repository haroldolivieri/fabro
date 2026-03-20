Goal: # Plan: Add node-level model validation + missing catalog aliases

## Context

Running `fabro run` with `model="gpt-54"` on a workflow node fails at runtime with `LLM error: Not found on anthropic: model: gpt-54`. Two issues contribute:

1. `fabro validate` doesn't warn about unknown model names on **nodes** (only validates stylesheet models via `StylesheetModelKnownRule`)
2. The catalog is missing hyphenated aliases like `gpt-54` for `gpt-5.4` (only has `gpt54`)

## Step 1: Add hyphenated aliases to catalog

**File: `lib/crates/fabro-llm/src/catalog.json`**

| Model ID | Current aliases | Add |
|---|---|---|
| `gpt-5.4` (line 161) | `["gpt54"]` | `"gpt-54"` |
| `gpt-5.4-pro` (line 178) | `["gpt54-pro"]` | `"gpt-54-pro"` |
| `gpt-5.4-mini` (line 194) | `["gpt54-mini"]` | `"gpt-54-mini"` |

**File: `lib/crates/fabro-llm/src/catalog.rs`** — add alias resolution tests:
- `gpt_54_hyphenated_alias` → asserts `get_model_info("gpt-54")` resolves to `gpt-5.4`
- `gpt_54_pro_hyphenated_alias` → same for `gpt-54-pro`
- `gpt_54_mini_hyphenated_alias` → same for `gpt-54-mini`

Update insta snapshots (`cargo insta review`) for `gpt_5_4_in_catalog` and `gpt_5_4_pro_in_catalog`.

## Step 2: Add `NodeModelKnownRule`

**File: `lib/crates/fabro-validate/src/rules.rs`**

Add `NodeModelKnownRule` right after `StylesheetModelKnownRule` (after line 977). Mirrors the stylesheet rule but iterates nodes:

- Iterate `graph.nodes.values()`
- If `node.model()` is `Some` and `get_model_info()` returns `None` → emit `Severity::Warning`
- If `node.provider()` is `Some` and `Provider::from_str()` fails → emit `Severity::Warning`
- Set `node_id: Some(node.id.clone())` on each diagnostic

Register `Box::new(NodeModelKnownRule)` in `built_in_rules()` (line 33, after `StylesheetModelKnownRule`).

**Tests** (following existing `stylesheet_model_known_rule_*` pattern):
- `node_model_known_rule_valid_model` — known model, no warnings
- `node_model_known_rule_unknown_model` — unknown model, 1 warning with model name and node ID
- `node_model_known_rule_alias` — alias like `"opus"`, no warnings
- `node_model_known_rule_unknown_provider` — bad provider, 1 warning
- `node_model_known_rule_no_model_no_provider` — plain node, no warnings

## Step 3: Fix the workflow

**File: `fabro/workflows/implement/workflow.fabro` line 19**

Change `model="gpt-54"` to `model="gpt-54"` (now valid after Step 1 adds the alias). No change needed — it will just work.

## Verification

```bash
cargo test -p fabro-llm
cargo insta review          # accept updated snapshots
cargo test -p fabro-validate
cargo clippy --workspace -- -D warnings
cargo fmt --check --all
```

Then run `fabro validate fabro/workflows/implement/workflow.fabro` to confirm no warnings.


## Completed stages
- **toolchain**: success
  - Script: `command -v cargo >/dev/null || { curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && sudo ln -sf $HOME/.cargo/bin/* /usr/local/bin/; }; cargo --version 2>&1`
  - Stdout:
    ```
    cargo 1.94.0 (85eff7c80 2026-01-15)
    ```
  - Stderr: (empty)
- **preflight_compile**: success
  - Script: `cargo check -q --workspace 2>&1`
  - Stdout: (empty)
  - Stderr: (empty)
- **preflight_lint**: success
  - Script: `cargo clippy -q --workspace -- -D warnings 2>&1`
  - Stdout: (empty)
  - Stderr: (empty)
- **implement**: success
  - Model: claude-opus-4-6, 40.6k tokens in / 7.0k out
  - Files: /home/daytona/workspace/lib/crates/fabro-llm/src/catalog.json, /home/daytona/workspace/lib/crates/fabro-llm/src/catalog.rs, /home/daytona/workspace/lib/crates/fabro-validate/src/rules.rs


# Simplify: Code Review and Cleanup

Review all changed files for reuse, quality, and efficiency. Fix any issues found.

## Phase 1: Identify Changes

Run git diff (or git diff HEAD if there are staged changes) to see what changed. If there are no git changes, review the most recently modified files that the user mentioned or that you edited earlier in this conversation.

## Phase 2: Launch Three Review Agents in Parallel

Use the Agent tool to launch all three agents concurrently in a single message. Pass each agent the full diff so it has the complete context.

### Agent 1: Code Reuse Review

For each change:

1. Search for existing utilities and helpers that could replace newly written code. Use Grep to find similar patterns elsewhere in the codebase — common locations are utility directories, shared modules, and files adjacent to the changed ones.
2. Flag any new function that duplicates existing functionality. Suggest the existing function to use instead.
3. Flag any inline logic that could use an existing utility — hand-rolled string manipulation, manual path handling, custom environment checks, ad-hoc type guards, and similar patterns are common candidates.

Note: This is a greenfield app, so focus on maximizing simplicity and don't worry about changing things to achieve it.

### Agent 2: Code Quality Review

Review the same changes for hacky patterns:

1. Redundant state: state that duplicates existing state, cached values that could be derived, observers/effects that could be direct calls
2. Parameter sprawl: adding new parameters to a function instead of generalizing or restructuring existing ones
3. Copy-paste with slight variation: near-duplicate code blocks that should be unified with a shared abstraction
4. Leaky abstractions: exposing internal details that should be encapsulated, or breaking existing abstraction boundaries
5. Stringly-typed code: using raw strings where constants, enums (string unions), or branded types already exist in the codebase

Note: This is a greenfield app, so be aggressive in optimizing quality.

### Agent 3: Efficiency Review

Review the same changes for efficiency:

1. Unnecessary work: redundant computations, repeated file reads, duplicate network/API calls, N+1 patterns
2. Missed concurrency: independent operations run sequentially when they could run in parallel
3. Hot-path bloat: new blocking work added to startup or per-request/per-render hot paths
4. Unnecessary existence checks: pre-checking file/resource existence before operating (TOCTOU anti-pattern) — operate directly and handle the error
5. Memory: unbounded data structures, missing cleanup, event listener leaks
6. Overly broad operations: reading entire files when only a portion is needed, loading all items when filtering for one

## Phase 3: Fix Issues

Wait for all three agents to complete. Aggregate their findings and fix each issue directly. If a finding is a false positive or not worth addressing, note it and move on — do not argue with the finding, just skip it.

When done, briefly summarize what was fixed (or confirm the code was already clean).