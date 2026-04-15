**3984 passed, 1 timed out (pre-existing flaky signal test), 182 skipped**. All changes are working correctly.

Let me provide a summary of all the changes made:

## Summary of Changes

### 1. Core Type Changes (`lib/crates/fabro-types/src/status.rs`, `lib.rs`)
- **`RunStatus` enum**: Replaced `Succeeded`, `Dead` with `Queued`, `Blocked`, `Completed`, `Cancelled`
- **`is_terminal()`**: Now `Completed | Failed | Cancelled`
- **`is_active()`**: Added `Queued` and `Blocked`
- **`can_transition_to()`**: Updated transition rules for all new variants
- **`Display`/`FromStr`**: Updated to serialize as `queued`, `blocked`, `completed`, `cancelled`
- **`BlockedReason` enum**: New enum with `HumanInputRequired` variant
- **`RunStatusRecord`**: Added `blocked_reason: Option<BlockedReason>` field

### 2. OpenAPI Spec (`docs/api-reference/fabro-api.yaml`)
- **`RunStatus`**: Added `blocked` between `running` and `paused`
- **`InternalRunStatus`**: Replaced `succeeded`/`dead` with `queued`/`blocked`/`completed`/`cancelled`
- **`BlockedReason`**: New schema with `human_input_required`
- **`RunStatusResponse`**: Added `blocked_reason` field
- **`StoreRunSummary`**: Added `blocked_reason` field
- **`RunStatusRecord`**: Added `blocked_reason` field
- **`BoardColumn`**: Changed from `working/initializing/review/merge` to `working/blocked/review/merge`

### 3. Store/Projection (`lib/crates/fabro-store/`)
- **`run_state.rs`**: 
  - `InterviewStarted` now sets `status=Blocked` with `blocked_reason=HumanInputRequired`
  - `InterviewCompleted/Timeout/Interrupted` return to `Running` when no pending interviews remain
  - `RunFailed` with `reason=Cancelled` projects to `Cancelled` status
  - `build_summary()` populates `blocked_reason`
- **`types.rs`**: `RunSummary` gained `blocked_reason: Option<BlockedReason>` field
- **5 new tests**: interview blocked/unblocked, pause/unpause isolation, cancelled projection, queued serialization round-trip

### 4. Server (`lib/crates/fabro-server/`)
- **Board columns**: Changed to `working/blocked/review/merge`
  - `Running | Paused` → `working`
  - `Blocked` → `blocked`
  - `Completed` → `merge`
  - `Submitted/Queued/Starting/Failed/Cancelled/Removing` → off-board
- **`api_status_from_workflow()`**: Direct 1:1 mapping (no more special `Failed+Cancelled→Cancelled` remap)
- **`update_live_run_from_event()`**: `InterviewStarted` → `Blocked`; interview resolution → `Running`
- **`should_reconcile_run_on_startup()`**: Added `Blocked`
- **`cancel_run()`**: Added `Blocked` to cancellable statuses
- **All `RunStatusResponse` constructors**: Added `blocked_reason: None`
- **Demo**: Updated board columns; replaced `Initializing` with `Working`
- **Tests**: Updated all status assertions for new mapping

### 5. CLI (`lib/crates/fabro-cli/`)
- **`wait.rs`**: `Completed` is success exit state; missing status → `Failed`; display label "Completed"/"Cancelled"
- **`list.rs`**: Added `Blocked`, `Queued`, `Cancelled` color handling; removed `Dead`
- **`server_runs.rs`**: Fallback status → `Failed` (was `Dead`)
- **`attach.rs`**: `Completed` is success exit code
- **`runner.rs`**: Worker title "completed" (was "succeeded")
- **`dump.rs`**: Added `blocked_reason` to test fixture
- **All snapshot tests**: Updated "Succeeded"→"Completed", "succeeded"→"completed"

### 6. Workflow Engine (`lib/crates/fabro-workflow/`)
- **`finalize.rs`**: `Success/Skipped/PartialSuccess` → `RunStatus::Completed`
- **`resume.rs`**: Check for `Completed` (was `Succeeded`)
- **`run_lookup.rs`**: Fallback → `Failed` (was `Dead`)

### 7. Web UI (`apps/fabro-web/`)
- **`runs.ts`**: 
  - `ColumnStatus` → `working/blocked/review/merge`
  - `RunStatus` → added `queued/blocked/completed/cancelled`; removed `succeeded/dead`
  - `RunSummaryResponse` → added `blocked_reason`
- **`runs.tsx`**: Updated column styles; added interview events to `STATUS_EVENTS`
- **`runs.test.ts`**: Added tests for new statuses; verified `waiting`/`dead` are not recognized

### 8. Generated TypeScript Client (`lib/packages/fabro-api-client/`)
- Updated `RunStatus`, `InternalRunStatus`, `BoardColumn` enums
- Added `BlockedReason` model
- Updated `RunStatusResponse` and `RunStatusRecord` interfaces