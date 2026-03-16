Goal: # Style SVG output from `fabro graph` and API

## Context

SVGs from `fabro graph` use raw Graphviz defaults (black strokes, white bg, Times-serif). The docs SVGs look much better: teal nodes, gray edges, Helvetica font, transparent bg, dark mode CSS. The goal is to make `fabro graph` and the API `get_graph` endpoint produce the same styled output without building a layout engine.

## Approach: DOT defaults + SVG post-processing

### Step 1: `inject_dot_style_defaults(source: &str) -> String` in `graph.rs`

Find first `{`, insert after it:
```dot
    bgcolor="transparent"
    node [color="#357f9e", fontname="Helvetica", fontsize=12, fontcolor="#1a1a1a"]
    edge [color="#666666", fontname="Helvetica", fontsize=10, fontcolor="#666666"]
```
These are DOT defaults — per-node/edge attrs override them. Applies to both SVG and PNG.

### Step 2: `postprocess_svg(raw: Vec<u8>) -> Vec<u8>` in `graph.rs`

SVG-only. Two string operations:
1. Remove line containing `<polygon fill="white" stroke="none"` (the white background)
2. Insert dark mode `<style>` block after the `<svg ...>` closing `>`

### Step 3: Wire into `render_dot()` in `graph.rs`

- Call `inject_dot_style_defaults` on source before piping to `dot`
- Call `postprocess_svg` on output when format is SVG

### Step 4: Wire into `get_graph()` in `server.rs` (line ~1440, ~1467, ~1472)

- Wrap `dot_source` with `inject_dot_style_defaults`
- Wrap `output.stdout` with `postprocess_svg`

### Step 5: Wire into `get_run_graph()` in `demo/mod.rs` (line ~227, ~248, ~252)

Same two wraps.

### Step 6: Tests in `graph.rs`

- `inject_dot_style_defaults` inserts expected attrs
- `inject_dot_style_defaults` returns unchanged if no `{`
- `postprocess_svg` removes white background
- `postprocess_svg` injects dark mode style block
- Update `graph_valid_workflow_svg` to assert styled output

## Files to modify

1. `lib/crates/fabro-workflows/src/cli/graph.rs` — add 2 pub fns, modify `render_dot`, add tests
2. `lib/crates/fabro-api/src/server.rs` — 2-line change in `get_graph`
3. `lib/crates/fabro-api/src/demo/mod.rs` — 2-line change in `get_run_graph`

## Verification

1. `cargo test -p fabro-workflows -- graph` — unit tests pass
2. `cargo test -p fabro-api` — API tests pass
3. `cargo clippy --workspace -- -D warnings` — no warnings
4. `fabro graph fabro/workflows/implement/workflow.fabro -o /tmp/test.svg` — visually inspect:
   - Teal node strokes, gray edges, Helvetica font
   - No white background
   - Dark mode `<style>` block present
   - Compare with `docs/images/tutorial-plan-implement.svg`


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
  - Model: claude-opus-4-6, 37.6k tokens in / 7.8k out
  - Files: /home/daytona/workspace/lib/crates/fabro-api/src/demo/mod.rs, /home/daytona/workspace/lib/crates/fabro-api/src/server.rs, /home/daytona/workspace/lib/crates/fabro-workflows/src/cli/graph.rs


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