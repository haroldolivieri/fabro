## Summary

**Fixed 1 issue:**

- **Leaky abstraction** (line 409): `truncate_goal` was being called on `workflow_name`, which would incorrectly strip `#` characters and `Plan:` prefix from workflow names. Extracted a generic `truncate_str` helper for plain truncation and changed the `workflow_name` call site to use it. `truncate_goal` now delegates to `truncate_str` after doing its goal-specific cleanup.

**Noted but skipped (out of scope / false positive):**

- **Near-duplicate with `pr_title_from_goal`**: Valid finding — both functions do similar goal cleaning. However, they live in different crates (`fabro-cli` vs `fabro-workflows`) and extracting a shared utility into `fabro-util` is a larger refactor beyond the scope of this change.
- **Greedy `trim_start_matches('#')`**: Agent 2 noted this could strip `#` from non-heading contexts like `#channel-name`. In practice, goals in this system are user-provided task descriptions, not Slack references, and the existing test suite validates the expected behavior. The greedy approach is intentionally simpler and handles `###`+ headings that the `pr_title_from_goal` version misses. Not worth changing.
- **`"Plan:"` as a shared constant**: Only two uses across separate crates — not worth the abstraction overhead.