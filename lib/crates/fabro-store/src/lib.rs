use chrono::{DateTime, Utc};

mod artifact_store;
mod error;
mod keys;
mod run_state;
mod serializable_projection;
mod slate;
mod types;

pub use artifact_store::{ArtifactStore, NodeArtifact};
pub use error::{Error, Result};
pub use fabro_types::{
    EventEnvelope, NodeState, PendingInterviewRecord, RunBlobId, RunProjection, RunSummary, StageId,
};
pub use run_state::RunProjectionReducer;
pub use serializable_projection::SerializableProjection;
pub use slate::{
    AuthCode, ConsumeOutcome, Database, RefreshToken, RunDatabase, Runs, SlateAuthCodeStore,
    SlateAuthTokenStore,
};
pub use types::EventPayload;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ListRunsQuery {
    pub start: Option<DateTime<Utc>>,
    pub end:   Option<DateTime<Utc>>,
}
