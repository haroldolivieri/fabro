All tests pass. Let me now create a summary of all the changes made:

## Summary of Changes

### 1. Core Types (`lib/crates/fabro-types/src/status.rs`)
- **`RunStatus` enum**: Replaced `Succeeded`/`Dead` with `Queued`, `Blocked`, `Completed`, `Cancelled`
- **`BlockedReason` enum**: Added with initial value `HumanInputRequired`
- **`RunStatusRecord`**: Added `blocked_reason: Option<BlockedReason>` field; added `blocked()` constructor
- **`is_terminal()`**: Now returns true for `Completed | Failed | Cancelled`
- **`is_active()`**: Now includes `Queued`, `Blocked`
- **`can_transition_to()`**: Updated to support new states; any non-terminal → `Cancelled`
- **`Display`/`FromStr`**: Updated for all new variants
- Export `BlockedReason` from `lib.rs`

### 2. OpenAPI Spec (`docs/api-reference/fabro-api.yaml`)
- **`RunStatus`**: Added `blocked`, `removing`; removed `dead`/`succeeded` (now `completed`/`cancelled`)
- **`InternalRunStatus`**: Added `queued`, `blocked`, `completed`, `cancelled`; removed `succeeded`/`dead`
- **`BoardColumn`**: Changed from `working/initializing/review/merge` to `working/blocked/review/merge`
- **`BlockedReason`**: New enum with `human_input_required`
- Added `blocked_reason` field to `RunStatusResponse`, `RunStatusRecord`, and `StoreRunSummary`

### 3. Store Layer (`lib/crates/fabro-store/`)
- **`RunSummary`**: Added `blocked_reason` field
- **`RunProjection`**:
  - `InterviewStarted` → sets `Blocked` with `blocked_reason: HumanInputRequired`
  - `InterviewCompleted/Timeout/Interrupted` → returns to `Running` when no pending interviews remain
  - `RunFailed(reason=Cancelled)` → projects to `Cancelled` status
  - `RunCompleted` → projects to `Completed` (was `Succeeded`)
  - `build_summary()` includes `blocked_reason`
- **Tests**: 12 new tests covering blocked/unblocked transitions, cancelled projection, queued serialization, blocked_reason round-trips

### 4. Server (`lib/crates/fabro-server/`)
- **`board_column()`**: `Running/Paused` → `working`; `Blocked` → `blocked`; `Completed` → `merge`; others off-board
- **`board_columns()`**: Returns `working/blocked/review/merge`
- **`api_status_from_workflow()`**: Maps all new statuses including `Queued`, `Blocked`, `Cancelled`
- **`update_live_run_from_event()`**: `InterviewStarted` → `Blocked`; interview resolution → `Running`
- **`should_reconcile_run_on_startup()`**: Includes `Queued` and `Blocked`
- All `RunStatusResponse` constructors include `blocked_reason: None`
- Demo data updated (`Initializing` → `Blocked` for runs with questions)
- Test assertions updated for new status names

### 5. CLI (`lib/crates/fabro-cli/`)
- **`wait.rs`**: `Completed` is success exit; `Cancelled` displayed; missing status → `Failed`
- **`list.rs`**: Added colors for `Queued`, `Blocked`, `Cancelled`; removed `Dead`
- **`server_runs.rs`**: Missing status defaults to `Failed` instead of `Dead`
- **`attach.rs`**: Success check uses `Completed`
- All snapshot tests updated: `Succeeded` → `Completed`, `Dead` → removed

### 6. Workflow Engine (`lib/crates/fabro-workflow/`)
- `finalize.rs`: Success maps to `Completed` (was `Succeeded`)
- `resume.rs`: Completion check uses `Completed`
- `run_lookup.rs`: Missing status defaults to `Failed`

### 7. TypeScript/Web (`apps/fabro-web/`, `lib/packages/fabro-api-client/`)
- **Generated client**: Updated `BoardColumn`, `InternalRunStatus`, `RunStatus`, `RunStatusRecord`, `RunStatusResponse`; added `BlockedReason`
- **`data/runs.ts`**: `ColumnStatus` simplified to `working/blocked/review/merge`; `RunStatus` includes all canonical values; removed `dead`/`succeeded`/`waiting`
- **`routes/runs.tsx`**: Column styles updated; `STATUS_EVENTS` includes interview events
- **Tests**: New `isRunStatus` tests verify `blocked/paused/completed/cancelled` accepted, `dead/succeeded/waiting` rejected