All 390 tests pass and clippy is clean with zero warnings.

## Summary

Implemented the mapping of `reasoning_effort` to Anthropic's `output_config.effort` in three TDD steps:

1. **Added two failing tests** — `build_api_request_maps_reasoning_effort_to_output_config` (asserts `output_config == Some({"effort": "medium"})`) and `build_api_request_omits_output_config_when_no_reasoning_effort` (asserts `output_config.is_none()`).

2. **Added `output_config: Option<serde_json::Value>` field** to the `ApiRequest` struct (with `skip_serializing_if`), and set it to `None` in all existing construction sites. At this point Test B passed and Test A failed, proving the bug.

3. **Mapped `reasoning_effort` → `output_config`** in `build_api_request` by computing `output_config` from `request.reasoning_effort` (mirroring the OpenAI provider pattern) and wiring it into the `ApiRequest` construction. Both tests now pass.