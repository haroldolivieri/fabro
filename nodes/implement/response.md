All clean. Here's a summary of the two changes made:

1. **`retro.rs:196`** — Changed `envelope.get("node_label")` → `envelope.get("node_id")` so the HashMap key matches what the three lookup sites use (`node_id`).

2. **`retro.rs` test** — Changed `"node_label": "plan"` → `"node_label": "Plan"` and `"node_label": "code"` → `"node_label": "Code"` so the test data has differing `node_id`/`node_label` values. This ensures the test would have caught the original bug (lookups by `node_id` would miss if the map were keyed by `node_label`).