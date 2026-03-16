Clippy passes clean with no warnings.

## Summary

All three review agents found **no issues to fix**. The code is already clean:

- **Code Reuse**: No existing truncation or line-limiting utilities exist in the codebase — `tail_lines` is genuinely new functionality. ✅
- **Code Quality**: The `indent` parameter is justified (two different values at call sites), the abstraction boundary is clean (truncation + indentation are naturally coupled since indent must apply after truncation), and the constants are well-named. No parameter sprawl, no leaky abstractions. ✅
- **Efficiency**: Collecting `Vec<&str>` is trivially cheap (just borrowed pointers), and this is not a hot path (runs once per stage transition alongside multi-second LLM/SSH operations). The single-pass approach is idiomatic Rust. ✅

All 48 preamble tests pass and clippy is clean. **No changes needed.**