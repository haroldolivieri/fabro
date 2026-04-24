use fabro_hooks::{HookContext, HookEvent};
use fabro_types::BilledTokenCounts;

use super::types::{Concluded, FinalizeOptions, Retroed};
use crate::error::Error;
use crate::event::RunNoticeLevel;
use crate::git::MetadataStore;
use crate::outcome::{Outcome, OutcomeExt, StageStatus};
use crate::records::{Checkpoint, Conclusion, StageSummary};
use crate::run_dump::RunDump;
use crate::run_options::RunOptions;
use crate::run_status::{FailureReason, RunStatus, SuccessReason};
use crate::runtime_store::RunStoreHandle;
use crate::sandbox_git::git_push_host;
use crate::services::RunServices;

pub fn classify_engine_result(
    engine_result: &Result<Outcome, Error>,
) -> (StageStatus, Option<String>, RunStatus) {
    match engine_result {
        Ok(outcome) => {
            let status = outcome.status.clone();
            let failure_reason = outcome.failure_reason().map(String::from);
            let run_status = match status {
                StageStatus::Success | StageStatus::Skipped => RunStatus::Succeeded {
                    reason: SuccessReason::Completed,
                },
                StageStatus::PartialSuccess => RunStatus::Succeeded {
                    reason: SuccessReason::PartialSuccess,
                },
                StageStatus::Fail | StageStatus::Retry => RunStatus::Failed {
                    reason: FailureReason::WorkflowError,
                },
            };
            (status, failure_reason, run_status)
        }
        Err(Error::Cancelled) => (
            StageStatus::Fail,
            Some("Cancelled".to_string()),
            RunStatus::Failed {
                reason: FailureReason::Cancelled,
            },
        ),
        Err(err) => (
            StageStatus::Fail,
            Some(err.to_string()),
            RunStatus::Failed {
                reason: FailureReason::WorkflowError,
            },
        ),
    }
}

pub(crate) async fn build_conclusion_from_store(
    run_store: &RunStoreHandle,
    status: StageStatus,
    failure_reason: Option<String>,
    run_duration_ms: u64,
    final_git_commit_sha: Option<String>,
) -> Conclusion {
    let checkpoint = run_store
        .state()
        .await
        .ok()
        .and_then(|state| state.checkpoint);
    let stage_durations = run_store
        .list_events()
        .await
        .map(|events| crate::extract_stage_durations_from_events(&events))
        .unwrap_or_default();

    build_conclusion_from_parts(
        checkpoint.as_ref(),
        &stage_durations,
        status,
        failure_reason,
        run_duration_ms,
        final_git_commit_sha,
    )
}

fn build_conclusion_from_parts(
    checkpoint: Option<&Checkpoint>,
    stage_durations: &std::collections::HashMap<String, u64>,
    status: StageStatus,
    failure_reason: Option<String>,
    run_duration_ms: u64,
    final_git_commit_sha: Option<String>,
) -> Conclusion {
    let (stages, billing, total_retries) = if let Some(cp) = checkpoint {
        let mut stages = Vec::new();
        let mut retries_sum: u32 = 0;
        let mut billed_usage = Vec::new();

        for node_id in &cp.completed_nodes {
            let outcome = cp.node_outcomes.get(node_id);
            let retries = cp
                .node_retries
                .get(node_id)
                .copied()
                .unwrap_or(1)
                .saturating_sub(1);
            retries_sum += retries;

            if let Some(usage) = outcome.and_then(|o| o.usage.as_ref()) {
                billed_usage.push(usage.clone());
            }

            stages.push(StageSummary {
                stage_id: node_id.clone(),
                stage_label: node_id.clone(),
                duration_ms: stage_durations.get(node_id).copied().unwrap_or(0),
                billing_usd_micros: outcome
                    .and_then(|o| o.usage.as_ref())
                    .and_then(|usage| usage.total_usd_micros),
                retries,
            });
        }
        (
            stages,
            (!billed_usage.is_empty()).then(|| BilledTokenCounts::from_billed_usage(&billed_usage)),
            retries_sum,
        )
    } else {
        (vec![], None, 0)
    };

    Conclusion {
        timestamp: chrono::Utc::now(),
        status,
        duration_ms: run_duration_ms,
        failure_reason,
        final_git_commit_sha,
        stages,
        billing,
        total_retries,
    }
}

/// Write a finalize projection snapshot commit to the metadata branch.
///
/// This captures the final `run.json` projection state, including conclusion
/// and retro data. Best-effort: errors are logged as warnings.
pub async fn write_finalize_commit(run_options: &RunOptions, run_store: &RunStoreHandle) {
    let (Some(meta_branch), Some(repo_path)) = (
        run_options
            .git
            .as_ref()
            .and_then(|g| g.meta_branch.as_ref()),
        run_options.host_repo_path.as_ref(),
    ) else {
        return;
    };

    let git_author = run_options.git_author();
    let store = MetadataStore::new(repo_path, &git_author);
    let Ok(store_state) = run_store.state().await else {
        return;
    };
    let dump = RunDump::from_projection(&store_state);
    if let Err(e) =
        dump.write_to_metadata_store(&store, &run_options.run_id.to_string(), "finalize run")
    {
        tracing::warn!(error = %e, "Failed to write finalize commit to metadata branch");
        return;
    }

    let refspec = format!("refs/heads/{meta_branch}");
    git_push_host(
        repo_path,
        &refspec,
        &run_options.github_app,
        "finalize metadata",
    )
    .await;
}

async fn cleanup_sandbox(
    services: &RunServices,
    run_id: &fabro_types::RunId,
    workflow_name: &str,
    preserve: bool,
) -> std::result::Result<(), String> {
    let hook_ctx = HookContext::new(
        HookEvent::SandboxCleanup,
        *run_id,
        workflow_name.to_string(),
    );
    let _ = services.run_hooks(&hook_ctx).await;
    if !preserve {
        services.sandbox.cleanup().await?;
    }
    Ok(())
}

/// FINALIZE phase: classify outcome, build conclusion, persist terminal state.
///
/// # Errors
///
/// Returns `Error` if persisting terminal state fails.
pub async fn finalize(retroed: Retroed, options: &FinalizeOptions) -> Result<Concluded, Error> {
    let Retroed {
        graph,
        outcome,
        run_options,
        duration_ms,
        services,
        retro: _,
    } = retroed;

    let (final_status, failure_reason, _run_status) = classify_engine_result(&outcome);
    let conclusion = build_conclusion_from_store(
        &services.run_store,
        final_status,
        failure_reason,
        duration_ms,
        options.last_git_sha.clone(),
    )
    .await;

    write_finalize_commit(&run_options, &services.run_store).await;

    if options.preserve_sandbox {
        let info = services.sandbox.sandbox_info();
        let message = if info.is_empty() {
            "sandbox preserved".to_string()
        } else {
            format!("sandbox preserved: {info}")
        };
        services
            .emitter
            .notice(RunNoticeLevel::Info, "sandbox_preserved", message);
    }
    if let Err(e) = cleanup_sandbox(
        &services,
        &options.run_id,
        &options.workflow_name,
        options.preserve_sandbox,
    )
    .await
    {
        tracing::warn!(error = %e, "Sandbox cleanup failed");
        services.emitter.notice(
            RunNoticeLevel::Warn,
            "sandbox_cleanup_failed",
            format!("sandbox cleanup failed: {e}"),
        );
    }

    Ok(Concluded {
        run_id: run_options.run_id,
        outcome,
        conclusion,
        pushed_branch: run_options.git.as_ref().and_then(|g| g.run_branch.clone()),
        graph,
        run_options,
        services,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use fabro_graphviz::graph::Graph;
    use fabro_store::Database;
    use fabro_types::settings::SettingsLayer;
    use fabro_types::{RunId, fixtures};
    use object_store::memory::InMemory;

    use super::*;
    use crate::event::{Emitter, StoreProgressLogger};
    use crate::pipeline::types::Retroed;
    use crate::run_options::RunOptions;

    fn test_run_id() -> RunId {
        fixtures::RUN_1
    }

    fn test_run_options(run_dir: &std::path::Path) -> RunOptions {
        RunOptions {
            settings:         SettingsLayer::default(),
            run_dir:          run_dir.to_path_buf(),
            cancel_token:     None,
            run_id:           test_run_id(),
            labels:           HashMap::new(),
            workflow_slug:    None,
            github_app:       None,
            host_repo_path:   None,
            base_branch:      None,
            display_base_sha: None,
            git:              None,
        }
    }

    fn test_store() -> Arc<Database> {
        Arc::new(Database::new(
            Arc::new(InMemory::new()),
            "",
            Duration::from_millis(1),
            None,
        ))
    }

    #[tokio::test]
    async fn finalize_persists_conclusion_in_projection() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("run");
        std::fs::create_dir_all(&run_dir).unwrap();
        let inner_store = test_store().create_run(&test_run_id()).await.unwrap();
        let run_store = inner_store;
        let emitter = Arc::new(Emitter::new(test_run_id()));
        let store_logger = StoreProgressLogger::new(run_store.clone());
        store_logger.register(&emitter);
        let services = RunServices::new(
            run_store.clone().into(),
            Arc::clone(&emitter),
            Arc::new(fabro_agent::LocalSandbox::new(
                std::env::current_dir().unwrap(),
            )),
            None,
            None,
            fabro_model::Provider::Anthropic,
            Arc::new(fabro_auth::EnvCredentialSource::new()),
        );
        let retroed = Retroed {
            graph: Graph::new("test"),
            outcome: Ok(Outcome::success()),
            run_options: test_run_options(&run_dir),
            duration_ms: 5,
            services,
            retro: None,
        };

        let concluded = finalize(retroed, &FinalizeOptions {
            run_dir:          run_dir.clone(),
            run_id:           test_run_id(),
            workflow_name:    "test".to_string(),
            preserve_sandbox: true,
            last_git_sha:     None,
        })
        .await
        .unwrap();
        store_logger.flush().await;

        assert_eq!(concluded.conclusion.status, StageStatus::Success);
    }
}
