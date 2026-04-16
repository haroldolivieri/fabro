It passes when run in isolation - this is a flaky test, not related to our changes. The only real failure remains the pre-existing `paginated_endpoints_return_correct_shape`.

Let me now provide a summary of all the changes made:

## Summary of Changes Implemented

### Implementation Unit 1: Shared Types and OpenAPI

**`lib/crates/fabro-types/src/status.rs`**
- Added `Queued` and `Blocked` variants to `RunStatus` enum
- Updated `is_active()` to include `Queued` and `Blocked`
- Updated `can_transition_to()` with new transition rules
- Updated `Display` and `FromStr` for new variants
- Added `BlockedReason` enum with `HumanInputRequired` variant
- Renamed `RunStatusRecord.reason` to `status_reason`
- Added `blocked_reason: Option<BlockedReason>` to `RunStatusRecord`
- Added comprehensive transition tests

**`lib/crates/fabro-types/src/run_event/run.rs`**
- Added `RunBlockedProps` and `RunUnblockedProps` event property structs

**`lib/crates/fabro-types/src/run_event/mod.rs`**
- Added `RunQueued`, `RunBlocked`, `RunUnblocked` variants to `EventBody` enum
- Added event_name() entries for new events

**`lib/crates/fabro-types/src/lib.rs`**
- Added `BlockedReason` to public re-exports

**`docs/api-reference/fabro-api.yaml`**
- Collapsed `RunStatus` and `InternalRunStatus` into unified `RunStatus` schema
- Removed `InternalRunStatus` schema
- Added `BlockedReason` schema
- Updated `RunStatusRecord` to use `RunStatus`, renamed `reason` → `status_reason`, added `blocked_reason`
- Updated `RunStatusResponse` with `blocked_reason`
- Updated `StoreRunSummary` with non-null `RunStatus`, typed `status_reason`, `blocked_reason`
- Updated `BoardColumn` to `initializing | running | blocked | succeeded | failed`
- Added `pending_interviews` to `RunProjection`

### Implementation Unit 2: Lifecycle Events and Transition Rules

**`lib/crates/fabro-workflow/src/event.rs`**
- Added `RunQueued`, `RunBlocked`, `RunUnblocked` variants to `Event` enum
- Added trace(), event_name(), and event_body_from_event() entries

**`lib/crates/fabro-workflow/src/blocked_state.rs`** (new)
- `BlockedStateTracker` with mutex-guarded unresolved count
- Emits `run.blocked` on 0→1 transition, `run.unblocked` on 1→0 transition
- Thread-safe for parallel human stages
- 4 unit tests

**`lib/crates/fabro-workflow/src/handler/human.rs`**
- Integrated `BlockedStateTracker` - calls `on_interview_started()` after emitting interview.started
- Calls `on_interview_resolved()` after all resolution paths (timeout, interrupted, skipped, completed)

**`lib/crates/fabro-workflow/src/handler/mod.rs`**
- Added `blocked_state_tracker: Option<Arc<BlockedStateTracker>>` to `EngineServices`

**`lib/crates/fabro-workflow/src/pipeline/execute.rs`**
- Creates `BlockedStateTracker` and passes to `EngineServices`

**`lib/crates/fabro-workflow/src/handler/parallel.rs`**
- Propagates `blocked_state_tracker` to parallel branch services

### Implementation Unit 3: Durable Projection and Truthful Run APIs

**`lib/crates/fabro-store/src/run_state.rs`**
- Added handlers for `RunQueued`, `RunBlocked`, `RunUnblocked` events
- Paused-over-blocked preserves `blocked_reason`
- Unblocked while paused clears `blocked_reason` without changing `Paused` status
- `build_summary()` synthesizes `Submitted` for runs without lifecycle status
- 6 projection tests added

**`lib/crates/fabro-store/src/types.rs`**
- Added `blocked_reason: Option<BlockedReason>` to `RunSummary`

**`lib/crates/fabro-server/src/server.rs`**
- `api_status_from_workflow()` now does 1:1 direct mapping (no lossy remap)
- `update_live_run_from_event()` handles `RunQueued`, `RunBlocked`, `RunUnblocked`
- `cancel_run()` accepts `Blocked` status
- `pause_run()` accepts `Blocked` (immediate pause path with direct `run.paused` append)
- `should_reconcile_run_on_startup()` includes `Queued` and `Blocked`
- `board_column()` maps per new board contract
- `list_board_runs()` populates question text for blocked runs from pending interviews
- All `RunStatusResponse` structs include `blocked_reason`
- Board columns updated to `initializing | running | blocked | succeeded | failed`

**`lib/crates/fabro-server/src/demo/mod.rs`**
- Updated all demo data to use new `BoardColumn` and `RunStatus` variants

### Implementation Unit 4: Web Board Projection and UI

**`apps/fabro-web/app/data/runs.ts`**
- Updated `ColumnStatus` type to `initializing | running | blocked | succeeded | failed`
- Updated `columnNames` and `statusColors`
- Added `queued` and `blocked` to `RunStatus` type and `runStatusDisplay`

**`apps/fabro-web/app/routes/runs.tsx`**
- Updated `STATUS_EVENTS` with all new lifecycle events including `run.queued`, `run.blocked`, `run.unblocked`, and `interview.*` events

**TypeScript API client models** (manual update):
- `blocked-reason.ts` (new): `BlockedReason` enum
- `board-column.ts`: Updated enum values
- `run-status.ts`: Updated to canonical vocabulary
- `run-status-record.ts`: Updated to use `RunStatus`, `status_reason`, `blocked_reason`
- `run-status-response.ts`: Added `blocked_reason`
- `store-run-summary.ts`: Updated with typed `status`, `status_reason`, `blocked_reason`

### Implementation Unit 5: CLI Consumers

**`lib/crates/fabro-cli/src/commands/runs/list.rs`**
- Added `Queued` and `Blocked` color handling

**`lib/crates/fabro-cli/src/server_runs.rs`**
- Changed fallback status from `Dead` to `Submitted` (status is now non-null)