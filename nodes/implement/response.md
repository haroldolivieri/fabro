All three checks pass:
- ✅ `cargo test -p fabro-workflows` — 179 passed, 0 failed
- ✅ `cargo clippy --workspace -- -D warnings` — clean
- ✅ `cargo fmt --check --all` — formatted

Here's a summary of all changes made:

### `lib/crates/fabro-workflows/src/graph/types.rs`
- **Step 2**: Added `selection()` accessor to `Node` impl (returns `"deterministic"` by default, reads from `selection` attribute)
- **Steps 1 & 2**: Added tests `node_selection_default` and `node_selection_random`

### `lib/crates/fabro-workflows/src/engine.rs`
- **Step 4**: Added `weighted_random()` function — weighted-random edge selection using `rand::thread_rng()`, treating `weight <= 0` as 1
- **Step 6a**: Added `pick_edge()` dispatcher — delegates to `weighted_random` for `"random"`, `best_by_weight_then_lexical` otherwise
- **Step 6b**: Added `selection: &str` parameter to `select_edge()`, replaced all 3 `best_by_weight_then_lexical` calls with `pick_edge`
- **Step 6c**: Updated both production call sites to pass `node.selection()`
- **Step 5**: Updated all 8 existing `select_edge` test calls to pass `"deterministic"` as 5th argument
- **Steps 3 & 4**: Added 4 tests for `weighted_random` (empty, single, zero-weight distribution, high-weight dominance)
- **Steps 5 & 6**: Added 2 tests for random selection via `select_edge` (`select_edge_random_returns_some_edge`, `select_edge_random_preferred_label_still_wins`)

### `lib/crates/fabro-workflows/src/validation/rules.rs`
- **Step 8**: Added `RandomSelectionNoConditionsRule` struct + `LintRule` impl — checks for `selection="random"` nodes with conditional outgoing edges
- **Step 8**: Registered in `built_in_rules()` vec
- **Steps 7 & 8**: Added 3 tests (`random_selection_no_conditions_clean`, `random_selection_with_conditions_errors`, `deterministic_selection_with_conditions_ok`)

### `docs/workflows/transitions.mdx`
- **Step 9**: Updated intro paragraph to mention deterministic-by-default with link to random selection
- **Step 9**: Added "Random selection" section with DOT example and `<Note>` about validation constraint

### `docs/reference/dot-language.mdx`
- **Step 9**: Added `selection` row to the "All nodes" attribute table