Everything passes. Here's the summary:

## Summary

**Fixed: Duplicate subprocess orchestration** (the main issue found across all three reviews)

`render_dot_svg` in `server.rs` was a near-copy of `render_dot` in `graph.rs` — both spawned a `dot` subprocess, piped DOT source to stdin, injected style defaults, and post-processed SVG output. The only difference was sync vs async and error types.

**Changes made:**

1. **`graph.rs`**: Made `render_dot` public (the single authoritative implementation). Made `inject_dot_style_defaults` and `postprocess_svg` private — they're implementation details of `render_dot`, not a two-phase protocol callers need to know about.

2. **`server.rs`**: Replaced 35 lines of duplicated subprocess code in `render_dot_svg` with a 10-line function that calls `render_dot` via `tokio::task::spawn_blocking` and converts the result to a `Response`. The demo handler (`demo/mod.rs`) already delegates to `render_dot_svg`, so no changes needed there.

**Skipped (not worth addressing):**
- Efficiency findings in `postprocess_svg` (line-by-line filtering, `insert_str` shift) — SVGs are ~KB and subprocess launch dominates; not worth the complexity.
- Extracting color constants — the coupling between DOT attrs and CSS selectors is inherent and co-locating the values wouldn't improve readability.