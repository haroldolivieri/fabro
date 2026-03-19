All clean. Formatting passes, clippy passes with no warnings, and all 117 tests in `fabro-util` pass.

**Summary:** The code is already clean — no changes needed. All three review agents confirmed the implementation is well-structured:
- **No code reuse issues** — no duplicated utilities
- **No quality issues worth fixing** — the one finding (typed vs untyped JSON map) adds complexity without meaningful benefit for this internal protocol detail
- **No efficiency issues** — the batched approach is a net improvement over the prior single-event design