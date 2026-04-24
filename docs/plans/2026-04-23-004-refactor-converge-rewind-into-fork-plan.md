---
title: "refactor: Converge rewind into fork with archive-after"
type: refactor
status: completed
date: 2026-04-23
---

# refactor: Converge rewind into fork with archive-after

## Overview

Collapse the rewind workflow operation into fork by treating rewind as `fork(source, target) + archive(source)`, wrapped in a new server-side endpoint `POST /runs/{id}/rewind` so the composition is atomic from the client's perspective and produces a single audit trail. Both operations produce new RunIds; the source run is never mutated in place. Delete the `RunRewound` event, `reset_for_rewind` projection logic, the `ensure_not_archived` guard specific to rewind, and the custom CLI event-relay plumbing.

Introduce a new `RunSupersededBy { new_run_id }` event emitted on the source when rewind archives it, so anyone reading the source's event stream can answer "why is this run archived?" without cross-correlating fork + archive events. Keep `fabro rewind` as a CLI verb — it's the semantic users reach for — but it becomes a thin wrapper around the new server endpoint.

The user-visible shift: `fabro rewind <ID> @3` now returns a new RunId and archives the source, instead of rewinding the source's branches in place. This is a semantic contract change for anyone scripting against rewind's old RunId-preservation behavior, not a pure refactor.

## Problem Frame

Rewind and fork share ~80% of their implementation (target resolution, timeline walk, branch plumbing, metadata snapshot construction) but diverge in one substantive way: rewind mutates the source run's refs in place, while fork creates a new run. That in-place mutation forces rewind to carry a large tail of special-case code:

- A dedicated `RunRewound` event
- `reset_for_rewind()` on the projection to unwind the source's terminal state so it can resume
- A `current_status` precondition check (`ensure_not_archived`) to prevent rewinding archived runs
- ~100 lines of CLI-side event relay logic (`reset_rewound_run_state`) that appends `RunRewound` + `CheckpointCompleted` + `RunSubmitted` to reconstitute the source's runnable state after the git refs move
- A server guard arm that clears `accepted_questions` on `RunRewound`

All of this exists solely to un-terminate the source run. If we archive the source and spawn a new run instead, none of it is needed — a new run starts clean by construction, and the source stays terminated.

This convergence was brainstormed conversationally on 2026-04-23 (no formal `docs/brainstorms/` document). The chosen approach is option 2 of three: rewind = fork + archive source. This preserves the user-facing distinction between fork (parallel continuation, source keeps running) and rewind (replace the path, source is abandoned), without maintaining two implementations.

## Requirements Trace

**Code Consolidation**
- R1. A single codepath creates the new run and its branches. No in-place ref mutation for rewind.

**CLI Behavior (Preserved & Changed)**
- R2. `fabro rewind <ID> <target>` archives the source run and returns a new RunId initialized at the target checkpoint.
- R3. `fabro fork <ID> [target]` continues to leave the source run untouched.
- R4. The `--list` and `--no-push` flags continue to work on both commands. `--no-push` has unchanged semantics (pure flag passthrough). `--list` is preserved as a capability but its implementation moves server-side — the endpoint wraps `build_timeline` (not `build_timeline_or_rebuild`), so a run with a missing metadata branch returns an empty timeline from the server where today's CLI would rebuild it. See Unit 2 for the accepted rebuild regression.

**Cleanup & Deletion**
- R5. `RunRewound` event, `RunRewoundProps`, `reset_for_rewind`, and the rewind-specific `ensure_not_archived` usage are removed from the codebase. The greenfield constraint lets us delete rather than deprecate.

**Regression Prevention**
- R6. No regression in timeline resolution (ordinal `@N`, `node`, `node@N`) or parallel-interior handling.
- R7. User-facing documentation that currently teaches in-place-rewind semantics is updated to match the new behavior (see Unit 6).

## Scope Boundaries

- **Not** adding provenance fields (`forked_from: Option<RunId>`) on forked runs. Covered for rewind by `RunSupersededBy` on the source; adding symmetric provenance on the new run is a separate follow-up covering both fork and rewind.
- Fork's surface IS changing: `POST /runs/{id}/fork` is added in Unit 2 alongside `POST /runs/{id}/rewind`, and `fabro fork` becomes a thin CLI wrapper over the new endpoint (mirrors the rewind split). Rationale: consistency (both are mutating git operations) and remote-client support. Unit 2 introduces the first git-touching HTTP endpoints in `fabro-server` and establishes the shared "open git Store from run's `working_directory`" pattern used by rewind, fork, and timeline endpoints.
- **Not** changing `build_timeline_or_rebuild` behavior or the rebuild-from-events path. The new `GET /runs/{id}/timeline` endpoint wraps `build_timeline`; it does not modify the underlying function.
- **Not** migrating stored `RunRewound` events — greenfield, no deployed instances.
- **Not** widening `operations::archive`'s precondition. Rewind inherits the "terminal status required" rule; non-terminal sources (Paused, Blocked, Running, etc.) must be canceled or allowed to finish before they can be rewound. This is a deliberate narrowing from today's behavior — see User Decisions log.

## Context & Research

### Relevant Code and Patterns

- `lib/crates/fabro-workflow/src/operations/fork.rs` — destination op; already accepts `Option<RewindTarget>` and defaults to latest checkpoint when `None`. Reuse unchanged.
- `lib/crates/fabro-workflow/src/operations/rewind.rs` — source of shared timeline helpers to extract (`RewindTarget`, `TimelineEntry`, `RunTimeline`, `build_timeline`, `find_run_id_by_prefix`, `run_commit_shas_by_node`, `load_parallel_map`, `detect_parallel_interior`, `read_projection_at_commit`, `backfill_run_shas`). Fork already imports from this module; extraction makes the dependency explicit.
- `lib/crates/fabro-workflow/src/operations/archive.rs` — `pub async fn archive(&Database, &RunId, Option<ActorRef>) -> Result<ArchiveOutcome, Error>`. Idempotent on already-archived runs (`ArchiveOutcome::AlreadyArchived`). Returns `Precondition` error if the run is still running.
- `lib/crates/fabro-cli/src/commands/runs/archive.rs` — CLI archive wrapper. Shows the `client.archive_run(&run_id)` HTTP pattern the new rewind handler will call.
- `lib/crates/fabro-cli/src/commands/run/fork.rs` — template for the new rewind handler. Same shape: resolve run, load state, build timeline, handle `--list`, call `fork()`, print result.

### Institutional Learnings

- No relevant `docs/solutions/` entries found for rewind/fork convergence.
- Memory note: greenfield app, no migration concerns — lets us delete `RunRewound` cleanly instead of leaving it as a stub for historical replay.

### External References

Not needed. This is an internal refactor with no external contract surfaces; timeline resolution and branch manipulation already have well-tested implementations in the repo.

## Key Technical Decisions

- **Both rewind and fork become server-side endpoints.** Add `POST /runs/{id}/rewind` AND `POST /runs/{id}/fork` to the fabro-api server. Rationale for moving fork alongside rewind: architectural consistency (both are mutating git operations; keeping one CLI-only and the other server-side creates a split that the second reviewer rightly flagged) and a single HTTP surface for future web-UI consumers. The rewind handler composes fork + archive + event append; the fork handler is a simpler wrapper around `operations::fork()`. Both share the working_directory/git-Store machinery. **Limitation honestly acknowledged:** both endpoints still require the server to have filesystem access to the run's original `working_directory` (a durable field on `RunSpec`). There is no override mechanism in this plan; 501 is a hard failure for runs whose original path isn't server-accessible. A truly-remote scenario (CLI, server, and repo on different hosts) is NOT solved by this plan — that requires a future follow-up (override mechanism, rebuild endpoint, or server-side checkout).

  Rewind handler flow:
  1. Reject if already archived (409 via `reject_if_archived`)
  2. Load source status; reject with 409 Conflict if not terminal (Succeeded/Failed/Dead)
  3. Open git Store from `spec.working_directory` (501 if inaccessible)
  4. spawn_blocking: call `operations::fork()` → new_run_id
  5. Call `operations::archive(source)` FIRST
  6. On archive OK → append `RunSupersededBy { new_run_id }` to source (only-on-archive-success invariant) → return 200
  7. On archive Err → return 207 Multi-Status with `archived: false, archive_error`; do NOT append RunSupersededBy

  Fork handler flow: steps 3–4 only; **emits no source-side event** (source run is untouched per R3). Returns `{ source_run_id, new_run_id, target }`. Provenance on the new run lives implicitly in its branch contents; a future `RunForkedFrom`-style audit event is Deferred to Follow-Up (applies symmetrically to fork + rewind).

  Benefits: consistent architecture, remote-client support via HTTP, atomicity (rewind), single audit event on source (rewind). Costs: two new endpoints (request/response types, OpenAPI additions, client wrappers), the 501 failure mode applies to both. Does introduce a new boundary in fabro-server: opening a git Store from inside a handler. Establishes the spawn_blocking + per-run-working_directory pattern for future server-side git work.

- **Add `RunSupersededBy { new_run_id }` event (supersedes deprecated `RunRewound`).** Lives in `fabro-types::EventBody` and the `fabro-workflow::Event` enum. Emitted on the source run only, by the rewind endpoint, AFTER `operations::archive` succeeds. Projection arm on `run_state.rs` sets `superseded_by: Option<RunId>` on `RunProjection` so consumers can answer "what replaced this run?" with a single projection read (no event-log replay). Rationale: audit trail was the primary justification for the server-side endpoint; the projection field makes that audit first-class for UI/CLI consumers.

- **Retry semantics: accept orphan-run cost; clients SHOULD NOT auto-retry.** `POST /runs/{id}/rewind` is not idempotent — each call mints a fresh RunId via `fork()`. Fabro has no idempotency-key infrastructure today, and adding it for one endpoint is scope creep. Rationale: orphan runs are a known, acceptable cost; single-shot semantics from the CLI wrapper avoids the common case. A future cross-cutting idempotency-key mechanism can apply retroactively. Documented in Unit 3's CLI error-path notes ("do not retry on network error; check server state; rewind may have succeeded").

- **Shared timeline logic moves to `lib/crates/fabro-workflow/src/operations/timeline.rs`.** Naming: `timeline` = read-side (timeline parsing, target resolution, prefix lookup), `fork` = write-side (branch creation, metadata snapshot write). Rationale: `rebuild_meta.rs` already imports `RunTimeline` and `build_timeline` from rewind.rs — the `rewind` name no longer describes what's in that file.

- **Rename `RewindTarget` → `ForkTarget`.** Done as part of the module extraction so downstream renames land in one commit. Rationale: the type is now shared between fork and rewind (which is itself a fork call), and keeping the old name would imply rewind is the primary owner.

- **Delete `RunRewound` entirely.** Variant on `Event`, `EventBody::RunRewound`, `RunRewoundProps`, `"run.rewound"` discriminant. Also delete `reset_for_rewind()` on `RunProjection` and its caller in `lib/crates/fabro-store/src/run_state.rs`. Rationale: in option 2 the source run is archived, not resurrected; there is no projection state to reset. Greenfield constraint lets us delete rather than deprecate.

- **Remove `RewindInput.current_status` and the old in-place-rewind's `ensure_not_archived` call.** Rationale: in the server-endpoint design, archived-source rejection happens at `reject_if_archived` (handler step 1, 409 Conflict) and non-terminal rejection happens at the explicit status pre-check (handler step 3, 409 Conflict). The old `RewindInput.current_status` precondition is subsumed. Other `ensure_not_archived` call sites (resume, etc.) stay untouched.

- **Keep distinct rewind vs fork CLI output text.** Rewind prints `"Rewound <source>... new run <new>"`; fork prints `"Forked <source> -> <new>"`. Both output the new RunId and a `fabro resume <new>` hint. Rationale: the archive-source side effect is invisible from the new-run's branches, so the message is how users learn their source was archived.

## Open Questions

### Resolved During Planning

- **Where do shared timeline helpers live?** → New `lib/crates/fabro-workflow/src/operations/timeline.rs` module.
- **Does `RewindTarget` get renamed?** → Yes, to `ForkTarget`, as part of the extraction.
- **Output text alignment with fork?** → Keep distinct. Rewind emphasizes the abandoned source; fork emphasizes the parallel continuation.
- **Archived source as rewind input?** → **Rejected with 409 Conflict** via `reject_if_archived` (mirrors archive/unarchive handlers). Users must `fabro unarchive <id>` first if they intend to rewind. This supersedes the earlier "Resolved" note that said archived-source rewinds would succeed as a no-op — that note was written for the CLI-orchestration shape and doesn't apply to the server-endpoint shape. The `ArchiveOutcome::AlreadyArchived` path is therefore unreachable from rewind; the reject-pattern fires first.

### User Decisions (recorded 2026-04-23)

**Decisions from the first pass (pre-adversarial review):**
- **Archive precondition: non-terminal sources?** → **Accept the narrowing.** Rewind now requires source to be Succeeded/Failed/Dead. Documented explicitly in Scope Boundaries.
- **Fork-then-archive half-success handling?** → **Both pre-check and graceful degradation.** Pre-check before fork; graceful degradation on post-fork archive failure.
- **Recovery scenario restructuring?** → **Split into two scenarios** (`rewind_recovers_metadata_from_real_run_state` + `fork_chain_rebuilds_metadata`).
- **Server-side endpoint vs. CLI-only?** → **Server-side composite endpoint.** Adds `POST /runs/{id}/rewind`; CLI becomes a thin wrapper.

**Decisions from the Unit 2 adversarial review:**
- **HTTP status code for partial success?** → **207 Multi-Status.** Archive-failure-after-fork returns 207 with `archived: false, archive_error: <msg>`.
- **TOCTOU race mapping (post-archive Precondition)?** → **Graceful degradation.** Treat as concurrent-mutation race, return 207 (same shape as transport failure). Not a server bug; not a 500.
- **Event ordering (RunSupersededBy vs archive)?** → **Archive first, RunSupersededBy second.** If archive fails, source is cleanly-terminal-with-missing-provenance (repairable) rather than "superseded-but-still-Succeeded" (misleading).
- **Idempotency?** → **Accept orphan-run cost; document in Key Technical Decisions.** No Idempotency-Key infrastructure. CLI is single-shot and does not auto-retry. Future cross-cutting idempotency mechanism can apply retroactively.
- **`superseded_by` projection field?** → **Add now.** `RunProjection.superseded_by: Option<RunId>` set by the RunSupersededBy event arm. Makes "what replaced this run?" a single projection read for future UI and `fabro ps` consumers.
- **Handler structure?** → **Operations-layer composite.** Business logic lives in new `operations::rewind` async function; handler is a 4-line delegator matching `archive_run`'s pattern. File `operations/rewind.rs` is repurposed, not deleted (Unit 4 updated accordingly).

**Decisions from the second external review (2026-04-24):**
- **Archived runs as rewind input?** → **Reject with 409 Conflict.** `reject_if_archived` fires at step 1; users must `fabro unarchive <id>` first. Removes the contradiction with the old "Resolved During Planning" text.
- **Event ordering invariant on failure?** → **Only emit `RunSupersededBy` if archive succeeded.** No supersede event on 207 path — preserves the ordering rationale and prevents the "superseded but still Succeeded" state the ordering was designed to avoid.
- **`AppState.repo_path` gap (P1-3)?** → **Keep server-endpoint; solve explicitly.** Handler reads `working_directory` from the run's `RunSpec` projection, opens a git Store at that path inside `spawn_blocking`. New 501 Not Implemented failure mode for runs whose working_directory isn't accessible from the server process (sandboxes, remote workers).
- **`superseded_by` plumbing?** → **Plumb through RunSummary + OpenAPI in Unit 2.** Honor the "helps fabro ps" claim by adding the field to `RunSummary`, the projection→summary mapping, and the OpenAPI schema.
- **Status code convention (412 vs 409)?** → **Use 409 Conflict** for both archived-source and non-terminal-source rejections. Matches fabro-server's consistent use of `StatusCode::CONFLICT`; error message disambiguates the two cases. No 412 in this plan.
- **Server-side timeline/list endpoint?** → **Add `GET /runs/{id}/timeline` to Unit 2.** Matches the mutating-rewind server-side move for web-UI parity; shares the working_directory/git-Store machinery with the rewind endpoint. CLI `--list` calls this endpoint instead of reading local git state.

**Decisions from the third external review (2026-04-24):**
- **Retry-after-partial-success mitigation text?** → **Rewritten to unambiguously forbid rewind-retry.** The risk-table mitigation now explicitly says "run `fabro archive <source>` manually; do NOT retry `fabro rewind`." Fork mints a fresh RunId each call; retry would orphan another run.
- **Key Technical Decisions handler-steps ordering?** → **Flipped to match the rest of the plan.** High-level summary now shows archive-first, RunSupersededBy-on-success, matching the detailed handler section and the User Decisions log.
- **Timeline rebuild semantics?** → **Accept the regression; document it.** Server endpoint wraps `build_timeline` only, not `build_timeline_or_rebuild`. Documented in Unit 2's timeline endpoint description and in Unit 6 (user docs). Follow-up issue for either server-side rebuild or an explicit rebuild endpoint.
- **`summary_to_api_run_summary` wire-level serializer?** → **Added to Unit 2 file list.** The function at `server.rs:2611` manually builds JSON for `fabro ps` consumers; without editing it, `superseded_by` would exist in types/OpenAPI but never reach the wire.
- **Fork architectural consistency (CLI-only vs server-side)?** → **Move fork server-side too.** Adds `POST /runs/{id}/fork` alongside rewind. Resolves the architectural split the reviewer flagged. Expands Unit 2 scope by one endpoint; Unit 3 now rewrites both `fabro rewind` and `fabro fork` as thin wrappers. Note: the original "supports remote CLI + remote server + remote repo scenarios" framing was overstated (see next review); the server still requires filesystem access to the run's stored `working_directory`.

**Decisions from the fourth external review (2026-04-24):**
- **501 recovery mechanism?** → **Accept as unrecoverable in this plan.** No override, no CLI-local fallback. The server must have filesystem access to the run's stored `working_directory` (a durable `RunSpec` field); if it can't, 501 is a hard failure. The earlier "checkout-to-local-path-and-retry" guidance was wrong — a caller-side checkout at a different path doesn't change the stored value. The "remote CLI/server/repo on different hosts" benefit claim is walked back. Remote/sandbox recovery is a follow-up concern (override mechanism, rebuild endpoint, or server-side checkout).
- **Fork source-side audit event?** → **None in this plan.** Fork emits no source-side event (source is untouched per R3). Provenance on the new run lives implicitly in its branch contents. `RunForkedFrom` is Deferred to Follow-Up as a symmetric audit event applying to both fork and rewind.
- **Outside-git tests?** → **Rewrite or delete.** `rewind_outside_git_repo_errors` and `fork_outside_git_repo_errors` asserted a local-git precondition that no longer applies (both mutate paths + `--list` now go through the server). Unit 5 instructs either deletion or rewrite to assert a different failure condition.
- **Scope summary + Unit 6 docs under-scoped?** → **Expanded.** Fork's user-facing docs (checkpoints.mdx:159, cli.mdx:582) are added to Unit 6's file list; the shared 501 limitation is documented once. API surface parity section now enumerates all three endpoints, both request/response schema pairs, the RunSummary field, and the new event wiring.

### Deferred to Implementation

- **Exact module visibility of timeline helpers.** Some helpers (`run_commit_shas_by_node`, `find_run_id_by_prefix_opt`) are `pub(crate)` or `pub(super)` today. Reclassify during the move based on who imports from outside `operations::`.
- **Whether to delete any timeline unit tests or move them unchanged.** The rewind-specific tests (`rewind_moves_metadata_ref`, `rewind_rejects_archived_runs`) go away with the op; timeline-resolution tests (`parse_target_ordinal`, `resolve_latest_visit`, `build_timeline_simple`, `parallel_interior_detection`, `find_run_id_prefix_match`) move to `timeline.rs`. If one bleeds into the other, sort it during extraction.

### Deferred to Follow-Up

- **Provenance field `forked_from: Option<RunId>` on forked run init events.** Useful for UI (showing the fork tree) and audit trails. Would apply symmetrically to both fork and rewind. Not required for this plan — `RunSupersededBy` on the source gives half the picture; the response body of both endpoints already returns `source_run_id`. File a follow-up issue after merge.

## High-Level Technical Design

> *This illustrates the intended control flow after convergence and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce. Module qualifiers below reflect the pre-extraction state; after Unit 1, `build_timeline` and `ForkTarget` live in `operations::timeline`, not `operations::rewind`.*

Today's control flow:

```
fabro rewind <ID> @3              fabro fork <ID> [@3]
         |                                |
         v                                v
 rewind CLI handler             fork CLI handler
  - build_timeline                - build_timeline
  - rewind() op                   - fork() op
    - move meta ref (in place)      - create new run branch
    - move run ref (in place)       - create new meta branch
    - emit RunRewound event         - write init + checkpoint snapshots
    - emit CheckpointCompleted      - return new RunId
    - emit RunSubmitted
    - reset projection state
  - print "To resume: fabro resume <SAME_ID>"
```

After convergence:

```
fabro rewind <ID> @3                  fabro fork <ID> [@3]
         |                                    |
         v                                    v
 rewind CLI (thin)                   fork CLI (thin)
  - --list: client.run_timeline(id)    - --list: client.run_timeline(id)
  - mutate: client.rewind_run(...)     - mutate: client.fork_run(...)
         |                                    |
         +----- both CLIs are ----------------+
         |     thin HTTP clients              |
         v                                    v

 POST /runs/{id}/rewind (server)     POST /runs/{id}/fork (server)
  - reject_if_archived (409)           - reject_if_archived (409)
  - check terminal status (409)        - open git Store at working_dir
  - open git Store at working_dir      - spawn_blocking: fork() op (501 if inacc)
  - spawn_blocking: fork() op          - return 200 { source, new, target }
    (501 if working_dir inacc)
  - archive(source) [FIRST]
  - on archive OK: append RunSupersededBy [SECOND, only on success]
  - return 200 (archive ok) | 207 (archive failed; no supersede)

 GET /runs/{id}/timeline (server)
  - open git Store at working_dir (501 if inacc)
  - spawn_blocking: build_timeline (NOT _or_rebuild — see Unit 2 note)
  - return 200 with Vec<TimelineEntryResponse>
```

The shared `fork()` op is the only code that creates runs, moves refs, or writes metadata snapshots. Rewind's differentiator is a server-side composite endpoint that adds a source-status pre-check, appends `RunSupersededBy` for audit, and archives the source. Fork continues to work exactly as today.

## Implementation Units

- [x] **Unit 1: Extract timeline module and rename RewindTarget → ForkTarget**

**Goal:** Move all timeline-reading logic out of `operations/rewind.rs` into a new `operations/timeline.rs` module. Rename `RewindTarget` to `ForkTarget` in the same pass so downstream callers update once.

**Requirements:** R1 (consolidate shared code), R6 (no regression in timeline resolution)

**Dependencies:** None — this is a pure code move.

**Files:**
- Create: `lib/crates/fabro-workflow/src/operations/timeline.rs`
- Modify: `lib/crates/fabro-workflow/src/operations/mod.rs` (add `mod timeline;`, re-export from `timeline` instead of `rewind`)
- Modify: `lib/crates/fabro-workflow/src/operations/rewind.rs` (remove the extracted symbols; the `rewind()` function and its helpers stay for now)
- Modify: `lib/crates/fabro-workflow/src/operations/fork.rs` (update import: `use super::timeline::{ForkTarget, TimelineEntry, build_timeline};`)
- Modify: `lib/crates/fabro-workflow/src/operations/rebuild_meta.rs` (update imports from `rewind::` to `timeline::`)
- Modify: `lib/crates/fabro-cli/src/commands/run/rewind.rs` (update `RewindTarget` → `ForkTarget` and import path)
- Modify: `lib/crates/fabro-cli/src/commands/run/fork.rs` (update `RewindTarget` → `ForkTarget` and import path)
- Test: tests move with the code — no new test file

**Approach:**
- Symbols to move verbatim into `timeline.rs`: `RewindTarget` (renamed `ForkTarget`), `TimelineEntry`, `RunTimeline`, `build_timeline`, `backfill_run_shas`, `run_commit_shas_by_node`, `detect_parallel_interior`, `find_run_id_by_prefix`, `find_run_id_by_prefix_opt`, `load_parallel_map`, `read_projection_at_commit`
- Symbols that stay in `rewind.rs` (will be rewritten in Unit 2, not deleted): `RewindInput` (repurposed with new fields), `rewind()` (repurposed as async composite). `rewind_to_entry()` is deleted entirely in Unit 4 — it has no equivalent in the new design.
- The existing `#[cfg(test)] mod tests` block in `rewind.rs` splits: timeline-parsing and resolution tests (`parse_target_ordinal`, `parse_target_latest_visit`, `build_timeline_simple`, `resolve_latest_visit`, `parallel_interior_detection`, `find_run_id_prefix_match`) move to `timeline.rs`; rewind-specific tests (`rewind_moves_metadata_ref`, `rewind_rejects_archived_runs`) stay for Unit 3 deletion.
- Visibility: `find_run_id_by_prefix_opt` is `pub(super)` today — keep `pub(super)` so it's reachable from `rebuild_meta.rs`. Adjust if rustc complains.

**Patterns to follow:**
- `lib/crates/fabro-workflow/src/operations/mod.rs` — existing re-export style (`pub use timeline::{...};`)
- No glob imports (CLAUDE.md rust import style)

**Test scenarios:**
- Happy path: `cargo build --workspace` succeeds after the move with zero behavior changes.
- Happy path: existing unit tests that move to `timeline.rs` pass unchanged against renamed `ForkTarget`.
- Edge case: `operations/rebuild_meta.rs` test `build_timeline_or_rebuild_rebuilds_missing_branch` continues to pass — verifies the new import wiring.

**Verification:**
- `cargo build --workspace` and `cargo nextest run -p fabro-workflow` both succeed.
- `rg "use .*rewind::(RewindTarget|TimelineEntry|RunTimeline|build_timeline|find_run_id_by_prefix)"` returns no matches — all call sites now import from `timeline`.
- Clippy passes: `cargo +nightly-2026-04-14 clippy --workspace --all-targets -- -D warnings`.

- [x] **Unit 2: Add `RunSupersededBy` event and three server endpoints (`POST /runs/{id}/rewind`, `POST /runs/{id}/fork`, `GET /runs/{id}/timeline`)**

**Goal:** Introduce the new audit event and the three server-side endpoints. All three share the "open git Store from run's `working_directory` inside `spawn_blocking`" machinery introduced here — solving that once enables all three. Moving both rewind AND fork server-side gives the architecture the consistency the second external review flagged; web UI consumers get a single HTTP surface. This does NOT solve the truly-remote scenario (CLI, server, and repo on different hosts) — the server must still have filesystem access to the run's original `working_directory`. That's a follow-up concern.

**Requirements:** R1 (single codepath), R2 (archive source + new RunId)

**Dependencies:** Unit 1 (needs `ForkTarget` in scope).

**Files:**
- Create event variant in `lib/crates/fabro-types/src/run_event/run.rs` — add `pub struct RunSupersededByProps { pub new_run_id: RunId, pub target_checkpoint_ordinal: usize, pub target_node_id: String, pub target_visit: usize }`. Model on `RunRewoundProps` (which is being deleted).
- Modify: `lib/crates/fabro-types/src/run_event/mod.rs` — add `RunSupersededBy(RunSupersededByProps)` variant to `EventBody`, `#[serde(rename = "run.superseded_by")]`, add `"run.superseded_by"` discriminant.
- Modify: `lib/crates/fabro-workflow/src/event.rs` — add `Event::RunSupersededBy { new_run_id, target_checkpoint_ordinal, target_node_id, target_visit }` variant, logging arm, discriminant, and `EventBody` conversion. Model on the existing `Event::RunRewound` shape (being deleted in Unit 5).
- Modify: `lib/crates/fabro-types/src/run_projection.rs` — add `pub superseded_by: Option<RunId>` field to `RunProjection`, serde-defaulted to `None`.
- Modify: `lib/crates/fabro-store/src/run_state.rs` — add `EventBody::RunSupersededBy(props) => self.superseded_by = Some(props.new_run_id);` arm. Single-line projection update; source's archived-status transition still comes from the separate `RunArchived` event per normal lifecycle.
- Modify: `lib/crates/fabro-types/src/run_summary.rs` — add `pub superseded_by: Option<RunId>` field to `RunSummary` (serde-defaulted).
- Modify: the projection→summary mapping (exact file TBD — check `fabro-server` or `fabro-store` for where `RunSummary` is built from `RunProjection`; set `summary.superseded_by = projection.superseded_by`).
- Modify: **`lib/crates/fabro-server/src/server.rs` `summary_to_api_run_summary` (line 2611)** — this function manually builds the JSON response shape via `serde_json::json!{...}` and is the wire-level serializer used by `fabro ps` (called from `:2758` and `:4874`). Without adding `"superseded_by": summary.superseded_by` to the emitted JSON, the field exists in storage/types/OpenAPI but never reaches clients. This was missed in the prior plan revision.
- Modify: `docs/api-reference/fabro-api.yaml` — single consolidated set of additions (all three endpoints + shared types):
  - `POST /runs/{id}/rewind` path with:
    - `RewindRequest` schema: `{ target: Option<String>, push: Option<bool> }` (push defaults to true server-side)
    - `RewindResponse` schema: `{ source_run_id, new_run_id, target, archived, archive_error?: String }`
  - `POST /runs/{id}/fork` path with:
    - `ForkRequest` schema: `{ target: Option<String>, push: Option<bool> }` (matches current CLI which sends `push: !args.no_push` at `lib/crates/fabro-cli/src/commands/run/fork.rs:42`)
    - `ForkResponse` schema: `{ source_run_id, new_run_id, target }` (no archived field — fork doesn't archive)
  - `GET /runs/{id}/timeline` path with `TimelineEntryResponse` schema matching today's `TimelineEntry` shape (ordinal, node_name, visit, run_commit_sha).
  - `RunSupersededByProps` event schema and `"run.superseded_by"` added to the SSE event-name enum.
  - `superseded_by: Option<RunId>` property added to the `RunSummary` schema (around line 3943).
- Create: `pub async fn rewind(&Database, &GitStoreFactory, &RewindInput, Option<ActorRef>) -> Result<RewindOutcome, Error>` in `lib/crates/fabro-workflow/src/operations/rewind.rs`. This is the file's new contents — replaces the old in-place-rewind function. Mirror the signature style of `operations::archive`. The function composes `operations::fork` (inside a `spawn_blocking` block) + `operations::archive` + `RunSupersededBy` event append (archive-first-then-supersede, only-on-archive-success).
- Rewrite the `RewindInput` struct in the same file. New fields: `{ run_id: RunId, target: Option<ForkTarget>, push: bool }`. Note the removed field: `current_status` is gone — the composite op loads status from the projection itself. The type name is preserved for call-site stability.
- Add a `RewindOutcome` enum: `Full { new_run_id: RunId, archived: bool }` (archive succeeded) and `Partial { new_run_id: RunId, archive_error: String }` (archive failed post-fork). Handler maps to 200/207 respectively.
- Create: server handler in `lib/crates/fabro-server/src/server.rs` — thin `async fn rewind_run(...)` delegator into `operations::rewind`, matching the 4-line pattern of `archive_run` (line 6448). Add route `.route("/runs/{id}/rewind", post(rewind_run))` next to `archive_run` / `unarchive_run` (see lines 1086-1087).
- Create: `pub async fn fork(...)` in a new `lib/crates/fabro-workflow/src/operations/fork_op.rs` or repurpose existing `operations/fork.rs` into an async wrapper. The sync `fork()` function becomes the inner (called inside `spawn_blocking`) and the new async `fork()` handles store opening + working_directory lookup + error mapping.
- Create: server handler `async fn fork_run(...)` in `server.rs`. Add route `.route("/runs/{id}/fork", post(fork_run))`. Returns `{ source_run_id, new_run_id, target }`. Status codes: 200 success, 400 bad target, 404 unknown run, 409 source archived, 501 working_directory inaccessible. **Fork emits no source-side event** (source is untouched per R3); provenance lives in the new run's branch contents only. A symmetric `RunForkedFrom` audit event is Deferred to Follow-Up.
- Create: `pub async fn timeline(...) -> Result<Vec<TimelineEntry>, Error>` in `lib/crates/fabro-workflow/src/operations/timeline.rs` (the new module from Unit 1). Async wrapper around the existing sync `build_timeline` — opens the git Store from the run's working_directory inside `spawn_blocking`. **Known regression accepted by this plan:** the server endpoint wraps `build_timeline` only, NOT `build_timeline_or_rebuild` (the rebuild-from-events fallback today's CLI uses at `lib/crates/fabro-workflow/src/operations/rebuild_meta.rs:124`). A run with a missing metadata branch will show an empty timeline from the server endpoint where today's CLI would rebuild it. Document in Unit 6 (user docs). Follow-up: either move rebuild server-side too, or add a `POST /runs/{id}/rebuild` endpoint users can call explicitly when they hit "no checkpoints found."
- Create: server handler in `lib/crates/fabro-server/src/server.rs` — thin `async fn run_timeline(...)` delegator. Add route `.route("/runs/{id}/timeline", get(run_timeline))`. Status codes: 200 with `Vec<TimelineEntryResponse>` on success; 404 for unknown run; 501 for inaccessible working_directory.
- Modify: `lib/crates/fabro-workflow/src/event.rs` — append_event support for `RunSupersededBy` via existing event append pathway.
- Modify: `lib/crates/fabro-client/src/client.rs` — add hand-written wrappers for all three new endpoints (`rewind_run`, `fork_run`, `run_timeline`) following the archive_run wrapper pattern.
- Test: unit tests for `operations::rewind` and `operations::timeline` in their respective test modules (axum-free, covers composite branches including the 501/working_directory-inaccessible path); plus thin handler tests for HTTP-layer behavior following existing archive/unarchive test patterns.

**Approach:**
- Server handler flow (pseudo-code, directional):
  1. Parse run ID from path; reject if archived (via `reject_if_archived`, mirrors archive/unarchive).
  2. Read body → `RewindRequest { target: Option<String>, push: Option<bool> }`.
  3. Load source status from projection; reject with **409 Conflict** if not `Succeeded/Failed/Dead` (matches fabro-server's consistent use of `StatusCode::CONFLICT` for state preconditions — see multiple callers in `server.rs`). Include the canonical precondition message.
  4. **Open the git `Store` by looking up the run's working_directory.** `AppState` has no global `repo_path` — confirmed by grep: `pub struct AppState` at `server.rs:539` has no repo field. The handler loads the run's `RunSpec` from the projection store, reads `spec.working_directory` (`lib/crates/fabro-types/src/run.rs:58`), and opens a git `Store` at that path. **New precondition:** the server process must have filesystem access to the run's `working_directory`. If the path doesn't exist, isn't a git repo, or isn't accessible (e.g., the run was launched in a Daytona sandbox or on a remote worker whose filesystem isn't shared with the server), return **501 Not Implemented** with an honest error message explaining the limitation — NOT a "use the CLI" fallback suggestion (the CLI is a thin wrapper around this same endpoint; there is no CLI-local rewind/fork implementation after Unit 3). The Store must be Send + 'static so the whole git block can run inside `spawn_blocking`.
  5. **Wrap steps 5–6 in `tokio::task::spawn_blocking`** — `operations::fork` does sync libgit2 work including potential remote push, which can block for seconds. Precedent: `spawn_blocking` is the established pattern in `server.rs` (lines 1291, 1331, 1674, 1711, 4564). Running `fork()` directly on the async runtime stalls Tokio workers under load. The spawn_blocking return should carry the new_run_id back to async context.
  6. Inside spawn_blocking: build timeline (sync), resolve target (`None` defaults to latest checkpoint), call `operations::fork(...)` → `new_run_id`.
  7. Back on the async runtime: call `operations::archive(&state.store, &id, actor)` FIRST.
  8. On archive `Ok` → append `RunSupersededBy { new_run_id, ... }` to source's event stream, then return 200 with `{ source_run_id, new_run_id, target, archived: true }`. On archive `Err(Precondition)` (expected concurrent-mutation race where status changed between step 3 and step 7) or `Err(engine)` (transport/internal failure) → **return 207 Multi-Status** with `{ source_run_id, new_run_id, target, archived: false, archive_error: <message> }`. **Do NOT emit `RunSupersededBy` on archive failure** — emitting it would recreate the "superseded but still Succeeded" state the archive-first ordering exists to prevent. The response body still carries `new_run_id` so clients know about the new run; no source-side audit trail in this case, which is the honest representation of partial success. If the RunSupersededBy append itself fails after a successful archive, log the failure prominently; source is cleanly archived with missing provenance (repairable via follow-up manual append). **Invariant: `RunSupersededBy` is only on the event stream iff source is archived.**
- **Business logic should live in `operations::rewind`, not the handler.** Mirror the existing `archive_run` handler pattern (`server.rs:6448-6462`): a 4-line delegator into a `pub async fn rewind(...)` function in `fabro-workflow::operations`. Handler handles HTTP parsing, auth, and response shaping; the composite fork+archive+event-append flow lives in the ops layer and is unit-testable without axum. This changes Unit 4 from "delete `rewind.rs`" to "replace `rewind.rs` contents with the new composite op" — the file stays, its contents change. See `Files:` list below.

**Technical design:** *(directional)*

```
// Request body
struct RewindRequest {
    target: Option<String>,
    push:   Option<bool>,  // default true
}

// Response body
struct RewindResponse {
    source_run_id:  RunId,
    new_run_id:     RunId,
    target:         String,         // canonical resolved form, e.g. "@2" or "build@1"
                                    // (never None; if request.target was None, response carries
                                    //  the resolved latest-checkpoint form)
    archived:       bool,           // false iff archive step failed post-fork
    archive_error:  Option<String>, // present iff archived == false
}

// Status codes:
//   200 OK             — fork succeeded AND archive succeeded
//   207 Multi-Status   — fork succeeded AND archive failed (archived=false, archive_error set)
//   400 Bad Request    — target out of range, malformed request
//   404 Not Found      — run id unknown
//   409 Conflict       — source already archived OR source not terminal
//                        (matches fabro-server's consistent CONFLICT convention;
//                         error message disambiguates the two cases)
//   501 Not Implemented — run's working_directory not accessible from the server
//                         process (remote worker, container sandbox, missing path)
```

**Patterns to follow:**
- `lib/crates/fabro-server/src/server.rs:6448` (`archive_run`) and `:6456` (`unarchive_run`) — handler shape, `reject_if_archived` gate, actor extraction, `operations::archive` integration.
- `lib/crates/fabro-workflow/src/operations/fork.rs` — called as-is (sync, in-handler).
- `lib/crates/fabro-workflow/src/operations/archive.rs:53-95` — called as-is (async).
- `lib/crates/fabro-server/src/server.rs:6058` (`reject_if_archived`) — precondition pattern.
- `lib/crates/fabro-server/src/server.rs:6037-6053` (`denied_lifecycle_event_name`) — update: `RunSupersededBy` is a server-emitted event, so the rewind endpoint is its legitimate injection point. Comment should note this.

**Test scenarios:**
- Happy path: POST `/runs/{terminal_id}/rewind` with `{target: "@2"}` returns 200 with `{source, new, target, archived: true}`; source event log shows `RunArchived` then `RunSupersededBy` (archive-first ordering); source projection has `superseded_by: Some(new_run_id)`; source `RunSummary` exposes the same field; new run has its own initialized branches.
- Happy path (timeline): GET `/runs/{id}/timeline` returns 200 with a `Vec<TimelineEntryResponse>` matching the ordered checkpoints in the run's metadata branch.
- Happy path default: POST with no `target` field rewinds to the latest checkpoint.
- Happy path: POST with `push: false` skips remote push; archive still occurs.
- Error path: POST on a `Running` source → 409 Conflict with "must be terminal" message; NO new run created (pre-check blocks before fork).
- Error path: POST on an `Archived` source → 409 Conflict via `reject_if_archived`; no new run.
- Error path: POST on unknown run ID → 404.
- Error path: target `@99` out of range → fork error surfaces as 400 Bad Request; no archive attempt; source unchanged.
- Edge case: archive fails after fork (simulate via fault injection on the archive call — not via "archive source first", which is blocked by `reject_if_archived` before fork even runs) → returns **207 Multi-Status** with `archived: false, archive_error: <msg>`; new run is intact; source event log does **NOT** carry `RunSupersededBy` (only-on-archive-success rule).
- Edge case: source status changes between pre-check and archive (TOCTOU race, simulate with a concurrent event append) → archive returns `Err(Precondition)`; endpoint returns 207 (same shape as transport failure), NOT 500. Source event log does NOT carry `RunSupersededBy`.
- Edge case: `RunSupersededBy` append fails after archive succeeds (simulate storage error) → response is still 200 with `archived: true`; source is cleanly archived but provenance is missing in its event log. Log the append failure prominently; this is a repairable degradation.
- Error path: source already archived → `reject_if_archived` returns 409 before handler business logic runs; no fork attempt.
- Error path: source `working_directory` is not accessible (simulate by passing a path the server can't stat) → 501 Not Implemented with an honest error message explaining the limitation (no "use the CLI" fallback — the CLI is now a thin HTTP wrapper).
- Integration: full CLI → server → git path in a CLI-level or scenario test (covered in Unit 5).

**Verification:**
- `cargo nextest run -p fabro-server` passes.
- `cargo build -p fabro-api` regenerates types cleanly after OpenAPI changes.
- Conformance test `fabro-server` run-catches-spec-drift (per CLAUDE.md API workflow) passes.
- `rg -n 'run\.superseded_by' lib/crates/ docs/api-reference/` finds matching wire identifiers in at least `fabro-types`, `fabro-workflow`, and `fabro-api.yaml`.

- [x] **Unit 3: Rewrite both `fabro rewind` and `fabro fork` CLIs as thin wrappers around the new server endpoints**

**Goal:** Replace both CLIs' local git logic with calls to the new server endpoints, plus timeline-listing and output formatting. Output text continues to use "rewind"/"fork" vocabulary. Both CLI commands become small adapters: parse args, call the endpoint, render the response.

**Requirements:** R2 (rewind new behavior), R3 (fork unchanged user-facing behavior), R4 (`--no-push` unchanged; `--list` preserved as a capability with the accepted rebuild regression)

**Dependencies:** Units 1 and 2 (needs `ForkTarget`, all three server endpoints, and generated client methods).

**Files:**
- Modify: `lib/crates/fabro-cli/src/commands/run/rewind.rs` (full rewrite)
- Modify: `lib/crates/fabro-cli/src/commands/run/fork.rs` (full rewrite — becomes a thin wrapper over `client.fork_run`)
- Modify: `lib/crates/fabro-client/src/client.rs` — wrappers added in Unit 2 (`rewind_run`, `fork_run`, `run_timeline`); Unit 3 just consumes them.
- Test: `lib/crates/fabro-cli/tests/it/cmd/rewind.rs` and `lib/crates/fabro-cli/tests/it/cmd/fork.rs` (assertions rewritten in Unit 5)

**Approach — shared for both commands:**
- Delete the CLI-side helpers `reset_rewound_run_state`, `restored_checkpoint_event`, `run_event` (and their `RunRewoundProps`/`CheckpointCompletedProps`/`RunSubmittedProps` imports) from `rewind.rs`. They have no consumer after this unit.
- Keep `print_timeline` and `timeline_entries_json` — both CLIs use them; they now format data that arrived from the server, not data built locally.
- Both CLIs no longer call `operations::fork` or any git op directly; all git work is server-side.
- `--list` on both commands calls `GET /runs/{id}/timeline` rather than reading local git state.
- Origin validation (`ensure_matching_repo_origin`) still runs client-side when the CLI has local repo access; when it doesn't (fully-remote CLI), origin validation is skipped and the server's view is trusted.
- **Retry posture: single-shot for both mutate paths.** Neither `fabro rewind` nor `fabro fork` auto-retries on network error, timeout, or 5xx. On non-response failure, print a "check server state" message. Rationale: both endpoints mint fresh RunIds on each call; naive retry creates orphans.

**Approach — `fabro rewind` specifics:**
- `--list`: call `client.run_timeline(&run_id)` and format with `print_timeline` / `timeline_entries_json`.
- Mutate: parse target, build `RewindRequest { target, push: !args.no_push }`, call `client.rewind_run(&run_id, &req)`, handle 200 vs 207.
- Output text on 200: `"Rewound {source[:8]}; new run {new[:8]}"` followed by `"To resume: fabro resume {new[:8]}"`.
- Output text on 207: in addition to the above, print `"Warning: source not archived: {archive_error}. Run `fabro archive {source}` to finish."` — single-shot retry policy applies; do NOT auto-retry rewind.
- JSON output: echo the `RewindResponse` shape plus the HTTP status code so scripts can branch on 200 vs 207 without re-parsing.

**Approach — `fabro fork` specifics:**
- `--list`: call `client.run_timeline(&run_id)` (same endpoint as rewind's --list path). Format identically.
- Mutate: parse target, build `ForkRequest { target, push: !args.no_push }`, call `client.fork_run(&run_id, &req)`, handle response.
- Output text on 200: preserve today's fork message pattern — `"Forked {source[:8]} -> {new[:8]}"` followed by `"To resume: fabro resume {new[:8]}"` (matches current snapshot in `tests/it/cmd/fork.rs:57`).
- JSON output: echo the `ForkResponse` shape `{ source_run_id, new_run_id, target }` plus the HTTP status code.
- No partial-success case — fork doesn't archive, so there's no equivalent of the 207 path. Error paths: 400 bad target, 404 unknown run, 409 source archived, 501 working_directory inaccessible.
- `--no-push` translates to `push: false` in the request body (today's CLI does the same via `push: !args.no_push` at `fork.rs:42`); server honors it.

**Patterns to follow:**
- `lib/crates/fabro-cli/src/commands/run/fork.rs` — same shape for `--list` path.
- `lib/crates/fabro-cli/src/commands/runs/archive.rs:70` — `client.archive_run(&run_id).await` call site pattern, will mirror `client.rewind_run(&run_id, &req).await`.
- `lib/crates/fabro-client/src/client.rs:725` (existing `archive_run` wrapper) — location and style for the new `rewind_run` wrapper.

**Test scenarios:**
- Happy path: `fabro rewind <ID> @2 --no-push` on a succeeded run exits 0, stderr contains "Rewound" and the new RunId prefix; source run transitions to `Archived` (via server); source event log shows `RunArchived` then `RunSupersededBy`; new run branches exist locally after the server's fork push/update.
- Happy path (JSON): `--json` emits `{source_run_id, new_run_id, target, archived: true}` with both IDs resolvable.
- Edge case: `fabro rewind <ID>` (no target, no `--list`) prints the timeline via `GET /runs/{id}/timeline`; no mutation.
- Edge case: `fabro rewind <ID> --list` prints the timeline via `GET /runs/{id}/timeline`; source unchanged.
- Edge case: `--no-push` translates into `push: false` in the request body; server honors it.
- Error path: target `@99` out of range → server returns 400; CLI prints the error; source unchanged.
- Error path: source run is still running or paused → server returns **409 Conflict** with "must be terminal" message; CLI prints it clearly; no new run anywhere.
- Error path: source already archived → server returns 409 Conflict; CLI prints "run is archived; run `fabro unarchive` first and retry"; no new run.
- Edge case: server returns 207 Multi-Status with `archived: false, archive_error: "..."` → CLI prints the new RunId, the archive-failure warning with the `fabro archive <source>` hint, and exits 0 so scripts can still pick up the new RunId.
- Edge case: server returns 501 Not Implemented (working_directory inaccessible) → CLI prints a clear error: `"Server cannot access this run's working_directory. This is a hard limitation in the current release; a future version may support an override or rebuild mechanism."` Exits non-zero. No retry guidance — retrying won't help.
- Edge case: network error or timeout during POST /rewind → CLI exits non-zero with the "check server state" message; does NOT auto-retry.
- Integration: after `rewind <ID> @2`, `fabro ps` shows source as Archived with `superseded_by = new_run_id`, and the new RunId is resumable.

**Test scenarios — `fabro fork`:**
- Happy path: `fabro fork <ID> @2 --no-push` exits 0, stderr contains "Forked {source} -> {new}" and "To resume: fabro resume {new}" — matches today's snapshot at `tests/it/cmd/fork.rs:57`; request body sent to server has `push: false`.
- Happy path (default target): `fabro fork <ID>` with no target resolves to latest checkpoint server-side; CLI renders the response identically.
- Happy path (JSON): `--json` emits `{source_run_id, new_run_id, target}` (no `archived` field) plus HTTP status code.
- Edge case: `fabro fork <ID> --list` prints the timeline via `GET /runs/{id}/timeline` (same behavior as rewind --list); source unchanged.
- Edge case: `--no-push` translates into `push: false` in the ForkRequest body; server honors it.
- Error path: target `@99` out of range → server returns 400; CLI prints the error; no new run.
- Error path: source already archived → server returns 409 Conflict; CLI prints "run is archived; run `fabro unarchive` first" message; no new run.
- Edge case: server returns 501 Not Implemented (working_directory inaccessible) → CLI prints the same hard-limitation error as rewind; exits non-zero.
- Edge case: network error or timeout during POST /fork → CLI exits non-zero with "check server state" message; does NOT auto-retry (same reasoning as rewind — fork mints a fresh RunId per call).
- Integration: after `fork <ID> @2`, source run is unchanged (no `RunSupersededBy`, no archive); new RunId is resumable via `fabro resume <new>`.

**Verification:**
- `cargo nextest run -p fabro-cli` passes with Unit 5's updated assertions.
- Both `fabro rewind --help` and `fabro fork --help` output unchanged (args structs untouched).
- The CLI-snapshot tests `rewind_target_updates_metadata_and_resume_hint` and `fork_latest_prints_new_run_and_resume_hint` pass against new/unchanged output text.

- [x] **Unit 4: Delete RunRewound event, in-place rewind op, and projection reset plumbing**

**Goal:** Remove every code path that existed solely to support in-place rewind. Compile cleanly. Note: `rewind.rs` the file STAYS — Unit 2 replaced its contents with the new composite `operations::rewind` function. This unit deletes the old in-place `rewind()` body and associated wire-contract types, not the file.

**Requirements:** R5 (delete all RunRewound plumbing)

**Dependencies:** Units 1, 2, and 3 (nothing should import `rewind()` or reference `RunRewound` after those units; this unit verifies and deletes).

**Files:**
- Modify: `lib/crates/fabro-workflow/src/operations/rewind.rs` — confirm the OLD in-place `rewind()` body, the OLD `RewindInput.current_status` field, `rewind_to_entry`, and the `ensure_not_archived` precondition call are all gone. After Unit 2 the file contains only the NEW composite `pub async fn rewind(...)` and its helpers. The `RewindInput` type name survives — its fields are rewritten to `{ run_id, target, push }` (no `current_status`) in Unit 2.
- Modify: `lib/crates/fabro-workflow/src/operations/mod.rs` — update the `rewind::` re-export block to expose the new composite function (`pub use rewind::{rewind, RewindInput, RewindOutcome};`). `RewindInput` is the same name as before but a different struct shape. Old `RewindTarget`/`TimelineEntry`/`RunTimeline`/`build_timeline`/`find_run_id_by_prefix` re-exports move to `timeline::` per Unit 1.
- Modify: `lib/crates/fabro-workflow/src/event.rs` — delete `Event::RunRewound` variant, its logging arm (~line 613), its `"run.rewound"` discriminant (~line 1178), and its `EventBody::RunRewound` conversion (~line 1586)
- Modify: `lib/crates/fabro-types/src/run_event/mod.rs` — delete `EventBody::RunRewound(RunRewoundProps)` variant (~line 128), its `"run.rewound"` discriminant (~line 393), AND the `"run.rewound"` string-match arm at line 524. Confirmed sites: `rg -n 'run\.rewound|RunRewound' lib/crates/fabro-types/src/run_event/mod.rs` returns lines 127, 128, 393, 524 — all four must go.
- Modify: `lib/crates/fabro-types/src/run_event/run.rs` — delete `pub struct RunRewoundProps` (~lines 90-99)
- Modify: `lib/crates/fabro-types/src/run_projection.rs` — delete `pub fn reset_for_rewind(&mut self)` (~lines 134-149)
- Modify: `lib/crates/fabro-store/src/run_state.rs` — delete the `EventBody::RunRewound(_) => self.reset_for_rewind()` arm (~lines 170-172)
- Modify: `lib/crates/fabro-server/src/server.rs` — drop `| EventBody::RunRewound(_)` from the `reconcile_live_interview_state_for_event` match (~line 3172); update the comment at line 6043 about what flows through `append_run_event`
- Test: no new tests — deletion only. Tests validating the deletion are in Unit 5.

**Approach:**
- This unit is mostly deletion. Run it last among the code-change units.
- Keep `ensure_not_archived` and `archived_rejection_message` in `archive.rs` — they're used by resume and by server guards, not just rewind.
- `operations::rewind` (the file) stays and now holds the composite op from Unit 2 — DO NOT delete the file.
- Before deleting `RunRewoundProps`, confirm: `rg "RunRewound"` returns only the planned deletion sites (the new event is `RunSupersededBy`, not a rename).
- **Symmetry check for `reset_for_rewind` deletion.** That method clears 13 fields on `RunProjection`. Its deletion is safe only if forked-run initialization starts clean equivalently. `fork.rs:92-100` uses `RunProjection::default()` and populates only `spec`, `graph_source`, `start`, `sandbox` — strictly cleaner than `reset_for_rewind` produces. The one deliberate carry-over is `sandbox` (correct: forked run should share the source's sandbox environment). Walk the field list once before deleting to verify no drift has been introduced since this plan was written.
- **`reset_for_rewind` deletion is reversible via git history.** If a future op requires un-terminating a projection (manual recovery tooling, undo-archive flow), reintroduce the method from git rather than carrying dead code now.

**Patterns to follow:**
- Matches the clean-deletion pattern used in recent refactors — e.g., the approach in `docs/plans/2026-04-23-003-refactor-pr-commands-server-side-plan.md` for removing obsolete code paths.

**Test scenarios:**
- Test expectation: none — pure deletion. Correctness is proven by the full workspace compiling and by Unit 5's updated tests passing.

**Verification:**
- `cargo build --workspace` succeeds.
- `cargo nextest run --workspace` passes.
- `rg "RunRewound|reset_for_rewind|RunRewoundProps"` returns zero hits.
- `rg "\\brewind_to_entry\b|ensure_not_archived.*rewind|current_status.*RewindInput"` returns zero hits (the OLD in-place-rewind internals are gone). Notes: `operations::rewind` itself stays — it's the path of the NEW async composite op. `RewindInput` the type name also stays but with new fields `{ run_id, target, push }` — no `current_status`.

- [x] **Unit 5: Update tests for new rewind semantics**

**Goal:** Rewrite tests that asserted old in-place rewind behavior to assert the new fork-and-archive semantics. Split the recovery scenario into two focused scenarios. Delete tests for behavior that no longer exists.

**Requirements:** R2, R4, R6 (verify behavior preserved where it should be; verify changed where it should be)

**Dependencies:** Units 1, 2, 3, 4 complete.

**Files:**
- Modify: `lib/crates/fabro-cli/tests/it/cmd/rewind.rs` (rewrite assertions; preserve `--help` snapshot structure)
- Modify: `lib/crates/fabro-cli/tests/it/cmd/fork.rs` (rewrite assertions — CLI is now a thin HTTP wrapper; tests mock the server endpoint and assert the CLI renders responses correctly; drop local-git assertions)
- Modify: `lib/crates/fabro-cli/tests/it/cmd/resume.rs` — two tests use the old `rewind <source> ... resume <source>` (same RunId) pattern and will break under new semantics:
  - `resume_rewound_run_succeeds` (~line 61) — rewrite to capture the new RunId from rewind stderr/JSON and resume *that* id.
  - `resume_detached_does_not_create_launcher_record` (~line 125) — same pattern; same rewrite.
- Modify: `lib/crates/fabro-cli/tests/it/scenario/recovery.rs` — delete `rewind_and_fork_recover_missing_metadata_from_real_run_state` and split into two focused scenarios:
  - `rewind_recovers_metadata_from_real_run_state` — run a workflow, fork it, rewind the fork (new endpoint), verify the new-from-rewind run has the correct metadata and resumability.
  - `fork_chain_rebuilds_metadata` — run a workflow, fork, fork the fork, verify metadata reconstruction across the chain (no rewind involved).
- Modify: `lib/crates/fabro-store/src/run_state.rs` — delete any test that seeded a `RunRewound` event (none found in grep, but re-verify during implementation)

**Approach:**
- In `tests/it/cmd/rewind.rs`:
  - `rewind_outside_git_repo_errors` — **rewrite or delete.** The current test (`rewind.rs:41`) exercises `fabro rewind <id> --list` outside a git repo and expects a local-git-absent failure. After Unit 3, `--list` calls `GET /runs/{id}/timeline` over HTTP; outside-git is no longer an error for the list path. Decide: (a) delete the test if no meaningful assertion remains, or (b) rewrite to assert the CLI error message surfaced when the server is unreachable / the run ID is unknown. Same applies to `fork_outside_git_repo_errors` at `tests/it/cmd/fork.rs:41` — the fork mutate path is also now server-side, so outside-git isn't the failure mode anymore.
  - `rewind_list_prints_timeline_for_completed_git_run` — rewrite to mock `GET /runs/{id}/timeline` rather than reading local git; assert the CLI renders the server response identically to today's output.
  - `rewind_target_updates_metadata_and_resume_hint` — rewrite. New assertions: (1) command succeeds; (2) stderr includes "Rewound" and "To resume: fabro resume"; (3) the resume hint points at a new RunId (not `setup.run.run_id`); (4) source run is now Archived. Drop the old assertion that the source's metadata ref moved.
  - `rewind_preserves_event_history_and_clears_terminal_snapshot_state` — delete. This test asserted `run.rewound` + `checkpoint.completed` + `run.submitted` event append and projection reset, all of which no longer happen. Replace with a test that asserts BOTH sides explicitly: (1) source event log gains exactly two new events in order: `run.archived` then `run.superseded_by` (matches the archive-first ordering and the only-on-archive-success rule); (2) the new run's event log contains the expected init events in order (`run.submitted`, `checkpoint.completed` from the target checkpoint), with the exact expected event count. The original test's event-count-delta assertion is the kind of coverage that catches helper-function run_id-mixup bugs; preserve that discipline in the rewrite.
- In `tests/it/scenario/recovery.rs`:
  - Delete the existing `rewind_and_fork_recover_missing_metadata_from_real_run_state`.
  - Add `rewind_recovers_metadata_from_real_run_state` — runs a workflow, forks from a checkpoint, rewinds the fork (hits the new endpoint), captures the new RunId from the response/output, asserts metadata-branch + run-branch are present for the new RunId and that `fabro resume <new>` can pick up the work.
  - Add `fork_chain_rebuilds_metadata` — runs a workflow, forks, forks again; asserts metadata rebuild across the two-step fork chain. Contains no rewind, so no dependency on the new endpoint.
- Delete snapshot files referenced by deleted/rewritten tests: `cargo insta pending-snapshots` after test changes, then `cargo insta accept --snapshot <path>` per-file after verifying.

**Patterns to follow:**
- `lib/crates/fabro-cli/tests/it/cmd/fork.rs` — mirror fork's assertion style for new-RunId verification (confirmed present at implementation time).
- Snapshot-test discipline per CLAUDE.md: check `cargo insta pending-snapshots` before accepting.

**Test scenarios:**
- Happy path: `rewind_target_creates_new_run_and_archives_source` — run rewind, assert new RunId in output, assert source status is Archived, assert source's event log gains exactly two events in order: (1) `RunArchived`, (2) `RunSupersededBy` (archive-first ordering). Assert source `RunProjection.superseded_by == Some(new_run_id)` and `RunSummary.superseded_by == Some(new_run_id)`. Assert new run has init + checkpoint events.
- Edge case: `rewind_list_calls_timeline_endpoint` — `--list` calls `GET /runs/{id}/timeline`; no mutation, no local git access required.
- Edge case: `rewind_with_no_target_prints_timeline` — no-target invocation behaves like `--list`.
- Edge case: `rewind_no_push_skips_remote_but_still_archives` — `--no-push` translates to `push: false` on the request; source is still archived via the server endpoint.
- Error path: `rewind_target_out_of_range_does_not_archive` — bad target → server 400; source remains in original (non-archived) status; no new run branches created.
- Error path: `rewind_non_terminal_source_rejected` — source is still running/paused → server 409 Conflict with "must be terminal" message; no new run.
- Edge case: `rewind_graceful_degradation_on_archive_failure` — simulate archive failure via fault injection on the archive call path (NOT by pre-archiving the source, which `reject_if_archived` blocks at handler step 1). Expected: server returns 207; CLI prints new RunId with warning; exit code 0; source event log does NOT gain `RunSupersededBy` (only-on-archive-success invariant).
- Error path: `rewind_unknown_run_mutate` — `fabro rewind <unknown_id> @2` fails at `resolve_run` (prefix/id lookup before the main operation, CLI pattern at `commands/run/rewind.rs:40`). CLI prints "run not found: <unknown_id>" and exits non-zero. No server mutate call is made.
- Error path: `rewind_unknown_run_list` — `fabro rewind <unknown_id> --list` fails at the same resolution step; CLI prints the same "run not found" message; no timeline endpoint call.
- Happy path: `fork_cli_creates_new_run_and_prints_hint` — `fabro fork <ID> @2 --no-push` exits 0, stderr matches the "Forked X -> Y" pattern today's test at `tests/it/cmd/fork.rs:57` asserts; source unchanged (no events gained); new RunId is resumable.
- Happy path: `fork_cli_default_target_resolves_to_latest` — `fabro fork <ID>` (no target) resolves server-side to latest checkpoint; CLI output identical to the explicit `@N` case.
- Happy path (JSON): `fork_cli_json_output` — `--json` emits `{source_run_id, new_run_id, target}` (no `archived` field, distinguishing it from rewind's response).
- Edge case: `fork_cli_list_calls_timeline_endpoint` — `fabro fork <ID> --list` calls the shared timeline endpoint; no mutation.
- Edge case: `fork_cli_no_push_passthrough` — `--no-push` sends `push: false` in the ForkRequest body (mirrors today's `push: !args.no_push` behavior).
- Error path: `fork_cli_target_out_of_range` — bad target → server 400; CLI prints error; no new run.
- Error path: `fork_cli_archived_source_rejected` — source already archived → server 409; CLI prints "unarchive first" message.
- Error path: `fork_501_when_working_directory_inaccessible` — server can't reach the run's working_directory → 501 Not Implemented; CLI prints the same hard-limitation error as rewind; exits non-zero.
- Error path: `fork_cli_network_error_does_not_retry` — network failure during POST /fork → CLI exits non-zero with "check server state" message; no auto-retry.
- Error path: `fork_cli_unknown_run_mutate` — `fabro fork <unknown_id> @2` fails at `resolve_run` (same CLI resolution pattern at `commands/run/fork.rs:18`); CLI prints "run not found: <unknown_id>" and exits non-zero. No /fork call is made.
- Error path: `fork_cli_unknown_run_list` — `fabro fork <unknown_id> --list` fails at the same resolution step; same error message; no timeline endpoint call.
- Integration: `recovery.rs` scenarios above — rewind then resume the new RunId; fork chain rebuilds metadata.

**Verification:**
- `cargo nextest run -p fabro-cli -p fabro-server` passes.
- `cargo insta pending-snapshots` is empty after acceptance.
- No test references `RunRewound`, `reset_for_rewind`, or `ensure_not_archived` in a rewind-specific context.

- [x] **Unit 6: Update user-facing documentation for new rewind semantics**

**Goal:** Replace the "in-place destructive rewind" mental model in shipped user docs with the "rewind produces a new run from a prior checkpoint and archives the source" model. Add a changelog entry so users learn of the semantic shift.

**Requirements:** R7 (docs match behavior)

**Dependencies:** Units 1-5 complete and merged. Docs should describe the shipped behavior, not the planned behavior.

**Files:**
- Modify: `docs/execution/checkpoints.mdx` — two sections need updates:
  - Lines 140-159 (rewind): rewrite "resume from the same RunId" flow to "resume from the new RunId printed by rewind"; rewrite fork-vs-rewind contrast to "fork keeps both, rewind archives the source."
  - Line 159 (fork): fork now uses server-side HTTP; the "local independent copy" framing still holds semantically, but the new --list path goes through the server; note the shared 501 limitation for inaccessible working_directories.
- Modify: `docs/reference/cli.mdx` — three sections need updates:
  - Rewind CLI reference entry around line 584: remove "resets the original run in place" language; document new output format (`source_run_id` + `new_run_id` JSON fields, 207 partial-success response).
  - Fork CLI reference entry around line 582: update for server-backed behavior — `--list` now calls the server timeline endpoint; 501 is possible; no longer requires a local git repo for --list.
  - Shared 501 limitation note applicable to both commands.
- Create: `docs/changelog/<date>.mdx` — single entry announcing: (1) `fabro rewind` now creates a new run and archives the source (replacing in-place rewind); (2) both `fabro rewind` and `fabro fork` now call server endpoints (architectural change; same output for local runs, but runs whose `working_directory` the server can't see return 501); (3) new `RunSupersededBy` event + `superseded_by` projection field. Include a migration note for any scripts that parse rewind output.

**Approach:**
- Audit first: `rg -i "rewind|rewound" docs/ apps/` to confirm the file list. Ignore changelog history entries (they correctly describe behavior at their own date).
- Keep the `fabro rewind` CLI as the documented verb for "try from earlier checkpoint" — the semantic-name preservation is deliberate. Update the explanation of what it does, not the name.
- Mention in the docs that the source run is archived (not lost) and can be unarchived with `fabro unarchive` if needed.
- Document the accepted timeline regression: runs with a missing metadata branch will show an empty timeline from the server endpoint. If users need the old CLI rebuild behavior, a follow-up issue tracks either server-side rebuild or an explicit `POST /runs/{id}/rebuild` endpoint.
- Document the 501 failure mode honestly: when the server can't access a run's `working_directory` (remote worker, sandbox, missing path), rewind/fork/timeline all return 501. This is a hard limitation — there is no retry or workaround in this release. A follow-up may add an override or rebuild mechanism.

**Patterns to follow:**
- Existing `docs/changelog/*.mdx` format for the new entry.
- Mintlify docs conventions elsewhere in `docs/`.

**Test scenarios:**
- Test expectation: none — documentation-only change, no executable behavior.

**Verification:**
- Mintlify docs dev server renders the updated pages without warnings (`docker run ... mintlify dev` per CLAUDE.md).
- Manual read-through: the new text accurately describes the Unit 2 CLI output format and the source-is-archived behavior.
- `rg -i "in[ -]place|destructive" docs/execution/ docs/reference/` returns no rewind-related hits after the change.

## System-Wide Impact

- **Interaction graph:** Rewind AND fork are now HTTP calls from the CLI (`POST /runs/{id}/rewind`, `POST /runs/{id}/fork`); `--list` is also HTTP (`GET /runs/{id}/timeline`). The rewind endpoint atomically composes fork + archive server-side. Pre-check before fork eliminates the precondition half-success case; transport-level archive failure is handled by the endpoint returning `archived: false, archive_error: ...` so the CLI can surface the warning while still delivering the new RunId.
- **Error propagation:** Bad targets surface as server 400 on both `/rewind` and `/fork`. Archived source returns 409 on both (via `reject_if_archived`). **Rewind only**: non-terminal source returns 409 (rewind's pre-check in handler; disambiguated in error body). Fork does NOT pre-check terminal status — it preserves today's behavior of forking any non-archived source regardless of run status (per R3). Inaccessible `working_directory` returns 501 on all three endpoints (rewind/fork/timeline). **Rewind only**: post-archive Precondition errors (concurrent-mutation race) return 207 Multi-Status, same as transport failures — NOT 500. Archive errors are degradations, not bugs.
- **State lifecycle:** Source run transitions `Succeeded/Failed/Dead → Archived` via the existing archive pipeline. On success, `operations::archive` runs FIRST; then the server appends `RunSupersededBy { new_run_id }`. Event log reads `RunArchived, RunSupersededBy`. Ordering rationale: if RunSupersededBy fails after archive, source is cleanly archived with missing provenance (repairable). If we reversed, an archive failure after a supersede-append would leave source "superseded but still Succeeded" — a misleading projection state. Projection captures `superseded_by: Some(new_run_id)` so UIs/CLI can answer "what replaced this?" without event-log replay.
- **Event stream consumers:** `RunRewound` disappears from the event stream; `RunSupersededBy` appears. Any UI element, log filter, or downstream consumer that matched `"run.rewound"` will break. Per memory, this is greenfield with no deployed consumers — confirm during implementation that no docs/web consumers reference the old event name: `rg -i rewound docs/ apps/ lib/packages/` should return only documentation strings destined for update in Unit 6.
- **API surface parity:** `docs/api-reference/fabro-api.yaml` gets several additions: three new paths (`POST /runs/{id}/rewind`, `POST /runs/{id}/fork`, `GET /runs/{id}/timeline`) with their request/response schemas (`RewindRequest`/`RewindResponse`, `ForkRequest`/`ForkResponse`, `TimelineEntryResponse`), `RunSupersededByProps` event schema, `"run.superseded_by"` in the SSE event-name enum, and a `superseded_by` field on the `RunSummary` schema. Zero deletions — the spec does not currently reference rewound (verified: `rg -c rewound docs/api-reference/fabro-api.yaml` = 0). Regenerate the Rust client and TypeScript client per CLAUDE.md "API workflow" after spec edits.
- **Integration coverage:** The `recovery.rs` scenarios (post-split) are the main integration tests that cross the CLI / server / git boundary. Unit 5 covers them.
- **Unchanged invariants:** `operations::fork` (the sync git function), `operations::archive`, `operations::unarchive`, `operations::resume`, and the `ensure_not_archived` guards on non-rewind paths (e.g., resume) stay exactly as they are. The sync `operations::fork()` signature is unchanged — the new async wrapper/handler composes around it.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Users/scripts relying on rewind preserving the source RunId break silently. | Output text explicitly states `new run <id>` so the change is loud; `--json` output includes both `source_run_id` and `new_run_id` so scripts can adapt without parsing prose. User-facing docs are updated in Unit 6 so the documented contract matches new behavior. |
| Fork-succeeded-then-archive-failed leaves an extra run on the server. | Pre-check before fork eliminates the precondition-failure case. Transport-level archive failures produce `archived: false` in the response so the CLI can surface a warning while still giving the user the new RunId. **Cleanup: the user runs `fabro archive <source>` manually** (archive is idempotent). **Do NOT retry `fabro rewind`** — rewind is not idempotent; each call mints a fresh RunId via fork, so a retry would orphan another run. |
| Recovery scenario changes miss a subtle assertion. | Unit 5 splits into two focused scenarios and explicitly asserts new-RunId resumability and the event count delta. Run locally before merging. |
| Stale `insta` snapshots silently accept changed output. | Follow CLAUDE.md discipline: `cargo insta pending-snapshots` before `cargo insta accept`; accept per-file, never globally. |
| Archive precondition rejects non-terminal runs that `ensure_not_archived` used to allow. | Resolved via User Decisions: accept the narrowing. Documented in Scope Boundaries and the CLI error message; users who need to rewind a paused/blocked run cancel-or-kill it first. |
| OpenAPI spec drift after adding the endpoint and event. | `fabro-server` conformance test catches router/spec divergence. Regenerate both Rust and TypeScript clients immediately after spec edits; commit the generated updates in the same commit as the spec changes. |
| New `RunSupersededBy` event shape conflicts with fabro-web or external SSE consumers. | Search `apps/fabro-web` and any external consumer repos for `run\.rewound` and related event-name strings before merging. Currently greenfield, but a one-line grep keeps the assumption honest. |
| Server-side rewind/fork/timeline endpoints reject runs whose `working_directory` isn't server-accessible (501). | Accepted as a hard limitation in this plan. The CLI surfaces a clear, non-misleading error (no retry guidance). In practice most runs today are local and server+CLI share a filesystem; remote/sandbox runs need a follow-up (override mechanism, rebuild endpoint, or streaming-fork-from-client protocol). Not solved here. |
| `RunSupersededBy` omitted on 207 leaves source with no source-side audit trail of the rewind. | Accepted trade-off per event-ordering-invariant decision. Response body still carries `new_run_id`, so forward-direction audit (new→source) is available via the deferred `forked_from` provenance follow-up. Backward direction (source→new) is only available on archive success — which is the common case. |

## Documentation / Operational Notes

- User-facing docs teach the old in-place-rewind model explicitly and must be updated (see Unit 6):
  - `docs/execution/checkpoints.mdx:140-159` — documents `fabro rewind <RUN_ID>` followed by `fabro resume <RUN_ID>` using the same ID; contrasts rewind (destructive, resets original) against fork (independent copy).
  - `docs/reference/cli.mdx` — CLI reference entry for `fabro rewind`; lines around 584 contrast rewind vs. fork as in-place-reset vs. independent-copy.
  - Changelog entries: `docs/changelog/2026-03-14.mdx:26-34` and `docs/changelog/2026-03-15.mdx:8` are historical and can stay, but a new changelog entry for this semantic change is required.
- **OpenAPI spec + client regeneration.** Unit 2 adds three paths (`POST /runs/{id}/rewind`, `POST /runs/{id}/fork`, `GET /runs/{id}/timeline`) with their request/response schemas, plus `RunSupersededByProps` + `"run.superseded_by"` event wiring, plus the `superseded_by` field on `RunSummary`. After spec edits, rerun `cargo build -p fabro-api` (progenitor regenerates Rust types + reqwest client) and `cd lib/packages/fabro-api-client && bun run generate` (openapi-generator regenerates TS client). The `fabro-server` conformance test catches spec/router drift — run it locally after the endpoints are wired.
- No rollout concerns — greenfield, no migration.

## Sources & References

- Conversational brainstorm on 2026-04-23 (this session). Option 2 selected: rewind = fork + archive source. Elevated to server-side composite endpoint per user decision during document-review.
- Related code:
  - `lib/crates/fabro-workflow/src/operations/fork.rs` (destination op, called server-side in Unit 2)
  - `lib/crates/fabro-workflow/src/operations/rewind.rs` (source of extraction + deletion)
  - `lib/crates/fabro-workflow/src/operations/archive.rs` (archive op, called server-side in Unit 2)
  - `lib/crates/fabro-cli/src/commands/run/rewind.rs` (CLI thin-wrapper rewrite target)
  - `lib/crates/fabro-cli/src/commands/run/fork.rs` (pattern template for `--list` path)
  - `lib/crates/fabro-server/src/server.rs` — archive_run (line 6448) and unarchive_run (line 6456) as server-handler template; route registration (line 1086); `reject_if_archived` precondition (line 6058)
  - `lib/crates/fabro-client/src/client.rs` — existing archive_run/unarchive_run wrappers as template for new `rewind_run` wrapper
  - `docs/api-reference/fabro-api.yaml` — OpenAPI spec, source of truth for new endpoint and event name
- Related plans:
  - `docs/plans/2026-04-22-003-refactor-lock-down-server-secrets-plan.md` (recent refactor precedent for wire-contract cleanup)
