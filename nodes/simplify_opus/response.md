All TypeScript tests pass too.

Let me summarize the changes I made:

## Summary of Changes

### Problem
The implementation stage introduced `Blocked` as a first-class managed run status (set on `InterviewStarted`, cleared on interview resolution) and changed cancelled runs from projecting as `status=failed` to `status=cancelled`. However, two areas were missed:

### Changes

#### 1. `lib/crates/fabro-server/src/server.rs` — 4 fixes

- **`cancel_run` handler**: Added `RunStatus::Blocked` to the cancellable statuses match. Without this, a run waiting at a human gate (now with `Blocked` status on the managed run) would return `409 Conflict: "Run is not cancellable."` instead of accepting the cancel.

- **`delete_run` handler**: Added `RunStatus::Blocked` to the non-terminal statuses that receive a full cancel grace period before worker termination.

- **System info active run count**: Added `RunStatus::Blocked` to the filter so blocked runs are counted as active in the `/system/info` endpoint.

- **Start/resume conflict check**: Added `RunStatus::Blocked` to the statuses that prevent starting a new engine process on a run, since a blocked run already has an active worker.

#### 2. `lib/crates/fabro-server/tests/it/scenario/lifecycle.rs` — 2 test fixes

- **`full_http_lifecycle_cancel`**: 
  - The cancel response status assertion now accepts either `"running"` or `"blocked"` (the managed run may or may not have received the `InterviewStarted` event by the time cancel is called).
  - Changed `wait_for_run_state` from expecting `("failed", "cancelled")` to `("cancelled", "cancelled")` to match the new canonical projection.

- **`cancel_at_human_gate_persists_cancelled_terminal_event`**: Changed `wait_for_run_status` from expecting `["failed"]` to `["cancelled"]` to match the new canonical status for cancelled runs.