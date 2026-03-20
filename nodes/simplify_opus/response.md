All tests pass and clippy is clean. 

**Summary: The code is already clean — no changes needed.**

The three review agents found:
- **No code reuse issues** — the one-liner JSON mapping is appropriately inline in each provider
- **No efficiency issues** — trivial cost on a network-bound code path  
- **One pre-existing quality observation** (stringly-typed `reasoning_effort`) that is out of scope for this change — it's a codebase-wide pattern that would require a multi-crate refactor to address properly