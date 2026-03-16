All checks pass. Here's a summary of the review and fixes:

## Summary

**3 issues found and fixed:**

1. **Code duplication eliminated** — The async DOT→SVG rendering logic was copy-pasted across `server.rs::get_graph` and `demo/mod.rs::get_run_graph` (spawn `dot -Tsvg`, pipe stdin, collect stdout, apply style defaults + SVG post-processing). Extracted a shared `render_dot_svg()` async helper in `server.rs` (as `pub(crate)`). The demo handler now calls `crate::server::render_dot_svg(dot_source).await` — a single line replacing ~25 lines of duplicated code.

2. **Unnecessary allocation removed** — `String::from_utf8_lossy(&raw).into_owned()` in `postprocess_svg` always clones the buffer, even when the input is valid UTF-8 (which Graphviz always outputs). Changed to `String::from_utf8(raw)` which reuses the original `Vec<u8>` backing buffer zero-copy, with a `unwrap_or_else` fallback for the impossible non-UTF-8 case.

3. **Redundant `format!` eliminated** — `format!("\n{DARK_MODE_STYLE}")` allocated a new `String` on every call just to prepend a newline. Folded the leading newline into the `DARK_MODE_STYLE` constant and replaced with a direct `insert_str(_, DARK_MODE_STYLE)`.

**Skipped (not worth fixing):**
- The `lines().filter().collect::<Vec<_>>().join("\n")` pattern in `postprocess_svg` — micro-optimization for ~200 lines of SVG that takes microseconds vs. the ~50ms `dot` subprocess. Not worth the readability cost.