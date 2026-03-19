## Summary

**One issue found and fixed:**

**Bug in `build_segment_batch`: non-object JSON values silently bypassed `"type": "track"` injection** (Code Quality finding). The original code parsed each line as `serde_json::Value`, then used `if let Some(map) = obj.as_object_mut()` to inject the type field — but `batch.push(obj)` was *outside* the `if let`, meaning non-object JSON (numbers, strings, arrays) would be added to the batch without the required `"type"` field.

**Fix:** Changed the parse target from `serde_json::Value` to `serde_json::Map<String, serde_json::Value>`. This:
- Rejects non-object JSON at parse time (falls through to the `Err`/warn branch)
- Eliminates the `if let` conditional entirely — the `"type"` injection always runs
- Is simpler (fewer lines, no conditional branching)

**Other findings reviewed and deemed clean:**
- No code reuse opportunities missed (no existing JSONL utilities, UUID usage is standard, base64 auth pattern is Segment-specific)
- `lines.is_empty()` guard in `spawn_sender` is correctly defensive (serialization could fail for all tracks)
- `reqwest::Client::new()` per call is acceptable (runs in a one-shot subprocess, matches panic sender pattern)
- `Vec<String>` + `join` is fine for typical 1-5 event batches
- `json!` import in tests is used