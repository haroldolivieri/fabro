//! GET /api/v1/runs/{id}/files — per-request coalescing and handler.
//!
//! This module exposes the per-run request-coalescing primitive consumed by
//! the Files Changed endpoint. Concurrent HTTP callers for the same run share
//! one materialization; concurrent callers for different runs proceed in
//! parallel. See Unit 4 of the Run Files Changed plan for design rationale.
//!
//! The materialization is deliberately driven by [`tokio::spawn`] rather than
//! polling a `Shared` future: the spawned task makes progress regardless of
//! whether any caller is still waiting, so an abandoned request cannot leave
//! orphan git subprocesses in the sandbox. All panics are caught and surfaced
//! as a 500 `ApiError`, and the registry entry is removed on task completion
//! so a follow-up request triggers a fresh materialization.

use std::collections::HashMap;
use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use axum::http::StatusCode;
use fabro_api::types::PaginatedRunFileList;
use fabro_types::RunId;
use futures_util::FutureExt;
use tokio::sync::{Mutex, watch};

use crate::error::ApiError;

/// Shared outcome of a single materialization. Wrapped in [`Arc`] so every
/// coalesced caller walks away with a cheap clone rather than an owned copy.
pub type ListRunFilesResult = std::result::Result<PaginatedRunFileList, ApiError>;

type Shared = Arc<ListRunFilesResult>;

/// Registry type held on `AppState`. Maps each `RunId` to the watch channel
/// that downstream callers subscribe to while a materialization is in flight.
pub type FilesInFlight = Arc<Mutex<HashMap<RunId, watch::Receiver<Option<Shared>>>>>;

/// Construct a fresh, empty `FilesInFlight` registry.
pub fn new_files_in_flight() -> FilesInFlight {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Run `materialize` at most once per `run_id`, sharing the result with any
/// concurrent callers that arrive while it is still in flight.
///
/// The spawned task owns the materialization. Dropping the returned future
/// only unsubscribes *this* caller — the task still runs to completion and
/// cleans itself up from the registry. Panics inside `materialize` are
/// caught and returned as an internal-server-error `ApiError` to every
/// concurrent caller; a subsequent call on the same `run_id` after the
/// panic triggers a fresh materialization (no poisoning).
pub async fn coalesced_list_run_files<F, Fut>(
    inflight: &FilesInFlight,
    run_id: &RunId,
    materialize: F,
) -> Shared
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ListRunFilesResult> + Send + 'static,
{
    let mut rx = {
        let mut guard = inflight.lock().await;
        if let Some(existing) = guard.get(run_id) {
            existing.clone()
        } else {
            let (tx, rx) = watch::channel::<Option<Shared>>(None);
            guard.insert(run_id.clone(), rx.clone());

            let inflight = Arc::clone(inflight);
            let run_id_cloned = run_id.clone();
            tokio::spawn(async move {
                let result = AssertUnwindSafe(async move { materialize().await })
                    .catch_unwind()
                    .await;
                let shared: Shared = match result {
                    Ok(value) => Arc::new(value),
                    Err(_) => Arc::new(Err(ApiError::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Run files materialization panicked.",
                    ))),
                };
                // Send before unregistering so a new receiver subscribed via
                // `rx.clone()` still sees the cached value via `borrow()`.
                let _ = tx.send(Some(shared));
                inflight.lock().await.remove(&run_id_cloned);
            });
            rx
        }
    };

    loop {
        let snapshot: Option<Shared> = rx.borrow_and_update().clone();
        if let Some(value) = snapshot {
            return value;
        }
        if rx.changed().await.is_err() {
            // Sender dropped without sending. Shouldn't happen in practice
            // because the spawned task always sends before dropping, but be
            // defensive.
            return Arc::new(Err(ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Run files materialization channel closed.",
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use fabro_types::RunId;
    use tokio::time::{Duration, sleep};

    use super::*;

    fn run_id(_name: &str) -> RunId {
        // RunIds are ULIDs, not arbitrary strings; each test just needs
        // distinct values.
        RunId::new()
    }

    fn new_registry() -> FilesInFlight {
        new_files_in_flight()
    }

    fn ok_response() -> PaginatedRunFileList {
        PaginatedRunFileList {
            data: Vec::new(),
            meta: fabro_api::types::RunFilesMeta {
                truncated:               false,
                files_omitted_by_budget: None,
                total_changed:           0,
                to_sha:                  None,
                to_sha_committed_at:     None,
                degraded:                None,
                degraded_reason:         None,
                patch:                   None,
            },
        }
    }

    #[tokio::test]
    async fn concurrent_calls_for_same_run_share_one_materialization() {
        let inflight = new_registry();
        let counter = Arc::new(AtomicUsize::new(0));
        let run = run_id("run_aaaaaaaaaaaaaaaaaaaaaaaaaa");

        let materialize = {
            let counter = Arc::clone(&counter);
            move || {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    sleep(Duration::from_millis(30)).await;
                    Ok(ok_response())
                }
            }
        };

        let mat_a = materialize.clone();
        let mat_b = materialize;
        let inflight_a = Arc::clone(&inflight);
        let inflight_b = Arc::clone(&inflight);
        let run_a = run.clone();
        let run_b = run.clone();

        let (a, b) = tokio::join!(
            tokio::spawn(async move { coalesced_list_run_files(&inflight_a, &run_a, mat_a).await }),
            tokio::spawn(async move { coalesced_list_run_files(&inflight_b, &run_b, mat_b).await }),
        );

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(a.unwrap().is_ok());
        assert!(b.unwrap().is_ok());
    }

    #[tokio::test]
    async fn different_run_ids_materialize_in_parallel() {
        let inflight = new_registry();
        let counter = Arc::new(AtomicUsize::new(0));
        let run1 = run_id("run_aaaaaaaaaaaaaaaaaaaaaaaaaa");
        let run2 = run_id("run_bbbbbbbbbbbbbbbbbbbbbbbbbb");

        let make = |counter: Arc<AtomicUsize>| {
            move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    sleep(Duration::from_millis(10)).await;
                    Ok(ok_response())
                }
            }
        };

        let i1 = Arc::clone(&inflight);
        let i2 = Arc::clone(&inflight);
        let m1 = make(Arc::clone(&counter));
        let m2 = make(Arc::clone(&counter));
        let (r1, r2) = tokio::join!(
            coalesced_list_run_files(&i1, &run1, m1),
            coalesced_list_run_files(&i2, &run2, m2),
        );

        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert!(r1.is_ok());
        assert!(r2.is_ok());
    }

    #[tokio::test]
    async fn panic_surfaces_as_internal_error_and_does_not_poison_future_calls() {
        let inflight = new_registry();
        let run = run_id("run_cccccccccccccccccccccccccc");

        let first = coalesced_list_run_files(&inflight, &run, || async {
            panic!("boom");
        })
        .await;
        assert!(first.is_err(), "expected panic to become error");
        assert_eq!(
            first.as_ref().as_ref().unwrap_err().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        // Give the spawned cleanup task a moment to remove the entry.
        sleep(Duration::from_millis(20)).await;

        // A subsequent call on the same run_id triggers a fresh materialization.
        let second =
            coalesced_list_run_files(&inflight, &run, || async { Ok(ok_response()) }).await;
        assert!(second.is_ok());
    }

    #[tokio::test]
    async fn sequential_calls_trigger_fresh_materialization() {
        let inflight = new_registry();
        let counter = Arc::new(AtomicUsize::new(0));
        let run = run_id("run_dddddddddddddddddddddddddd");

        for _ in 0..3 {
            let counter_inner = Arc::clone(&counter);
            let _ = coalesced_list_run_files(&inflight, &run, move || async move {
                counter_inner.fetch_add(1, Ordering::SeqCst);
                Ok(ok_response())
            })
            .await;
            // Yield to let the spawned task clean up the registry entry before
            // the next iteration.
            sleep(Duration::from_millis(10)).await;
        }

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
