use carry_core::CarryError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VenueError {
    #[error("bad venue message: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Core(#[from] CarryError),
}
