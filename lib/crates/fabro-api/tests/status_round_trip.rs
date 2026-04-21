use std::any::{TypeId, type_name};

use chrono::{TimeZone, Utc};
use fabro_api::types::{
    BlockedReason as ApiBlockedReason, RunControlAction as ApiRunControlAction,
    RunStatus as ApiRunStatus, RunStatusRecord as ApiRunStatusRecord,
    StatusReason as ApiStatusReason,
};
use fabro_types::status::{
    BlockedReason, RunControlAction, RunStatus, RunStatusRecord, StatusReason,
};
use serde::Serialize;
use serde_json::{Value, json};

#[test]
fn status_family_reuses_domain_types() {
    assert_same_type::<ApiRunStatus, RunStatus>();
    assert_same_type::<ApiStatusReason, StatusReason>();
    assert_same_type::<ApiBlockedReason, BlockedReason>();
    assert_same_type::<ApiRunControlAction, RunControlAction>();
    assert_same_type::<ApiRunStatusRecord, RunStatusRecord>();
}

#[test]
fn status_family_json_tokens_match_openapi() {
    for (api_status, workflow_status, expected) in [
        (ApiRunStatus::Submitted, RunStatus::Submitted, "submitted"),
        (ApiRunStatus::Queued, RunStatus::Queued, "queued"),
        (ApiRunStatus::Starting, RunStatus::Starting, "starting"),
        (ApiRunStatus::Running, RunStatus::Running, "running"),
        (ApiRunStatus::Blocked, RunStatus::Blocked, "blocked"),
        (ApiRunStatus::Paused, RunStatus::Paused, "paused"),
        (ApiRunStatus::Removing, RunStatus::Removing, "removing"),
        (ApiRunStatus::Succeeded, RunStatus::Succeeded, "succeeded"),
        (ApiRunStatus::Failed, RunStatus::Failed, "failed"),
        (ApiRunStatus::Dead, RunStatus::Dead, "dead"),
        (ApiRunStatus::Archived, RunStatus::Archived, "archived"),
    ] {
        assert_string_json(api_status, expected);
        assert_string_json(workflow_status, expected);
    }

    for (api_reason, workflow_reason, expected) in [
        (
            ApiStatusReason::Completed,
            StatusReason::Completed,
            "completed",
        ),
        (
            ApiStatusReason::PartialSuccess,
            StatusReason::PartialSuccess,
            "partial_success",
        ),
        (
            ApiStatusReason::WorkflowError,
            StatusReason::WorkflowError,
            "workflow_error",
        ),
        (
            ApiStatusReason::Cancelled,
            StatusReason::Cancelled,
            "cancelled",
        ),
        (
            ApiStatusReason::Terminated,
            StatusReason::Terminated,
            "terminated",
        ),
        (
            ApiStatusReason::TransientInfra,
            StatusReason::TransientInfra,
            "transient_infra",
        ),
        (
            ApiStatusReason::BudgetExhausted,
            StatusReason::BudgetExhausted,
            "budget_exhausted",
        ),
        (
            ApiStatusReason::LaunchFailed,
            StatusReason::LaunchFailed,
            "launch_failed",
        ),
        (
            ApiStatusReason::BootstrapFailed,
            StatusReason::BootstrapFailed,
            "bootstrap_failed",
        ),
        (
            ApiStatusReason::SandboxInitFailed,
            StatusReason::SandboxInitFailed,
            "sandbox_init_failed",
        ),
        (
            ApiStatusReason::SandboxInitializing,
            StatusReason::SandboxInitializing,
            "sandbox_initializing",
        ),
    ] {
        assert_string_json(api_reason, expected);
        assert_string_json(workflow_reason, expected);
    }

    assert_string_json(ApiBlockedReason::HumanInputRequired, "human_input_required");
    assert_string_json(BlockedReason::HumanInputRequired, "human_input_required");

    for (api_action, workflow_action, expected) in [
        (
            ApiRunControlAction::Cancel,
            RunControlAction::Cancel,
            "cancel",
        ),
        (ApiRunControlAction::Pause, RunControlAction::Pause, "pause"),
        (
            ApiRunControlAction::Unpause,
            RunControlAction::Unpause,
            "unpause",
        ),
    ] {
        assert_string_json(api_action, expected);
        assert_string_json(workflow_action, expected);
    }
}

#[test]
fn run_status_record_json_matches_openapi_shape() {
    let updated_at = Utc
        .with_ymd_and_hms(2026, 1, 2, 3, 4, 5)
        .single()
        .expect("fixed timestamp should be valid");
    let expected = json!({
        "status": "failed",
        "status_reason": "cancelled",
        "blocked_reason": "human_input_required",
        "updated_at": "2026-01-02T03:04:05Z"
    });

    let api_record = ApiRunStatusRecord {
        status: ApiRunStatus::Failed,
        status_reason: Some(ApiStatusReason::Cancelled),
        blocked_reason: Some(ApiBlockedReason::HumanInputRequired),
        updated_at,
    };
    assert_eq!(serde_json::to_value(&api_record).unwrap(), expected);
    let api_round_trip: ApiRunStatusRecord = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(api_round_trip.status, ApiRunStatus::Failed);
    assert_eq!(
        api_round_trip.status_reason,
        Some(ApiStatusReason::Cancelled)
    );
    assert_eq!(
        api_round_trip.blocked_reason,
        Some(ApiBlockedReason::HumanInputRequired)
    );
    assert_eq!(api_round_trip.updated_at, updated_at);

    let workflow_record = RunStatusRecord {
        status: RunStatus::Failed,
        status_reason: Some(StatusReason::Cancelled),
        blocked_reason: Some(BlockedReason::HumanInputRequired),
        updated_at,
    };
    assert_eq!(serde_json::to_value(&workflow_record).unwrap(), expected);
    let workflow_round_trip: RunStatusRecord = serde_json::from_value(expected).unwrap();
    assert_eq!(workflow_round_trip.status, RunStatus::Failed);
    assert_eq!(
        workflow_round_trip.status_reason,
        Some(StatusReason::Cancelled)
    );
    assert_eq!(
        workflow_round_trip.blocked_reason,
        Some(BlockedReason::HumanInputRequired)
    );
    assert_eq!(workflow_round_trip.updated_at, updated_at);
}

fn assert_same_type<T: 'static, U: 'static>() {
    assert_eq!(
        TypeId::of::<T>(),
        TypeId::of::<U>(),
        "{} should be the same type as {}",
        type_name::<T>(),
        type_name::<U>()
    );
}

fn assert_string_json<T: Serialize>(value: T, expected: &str) {
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        Value::String(expected.into())
    );
}
