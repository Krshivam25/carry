use carry_core::CarryError;
use thiserror::Error;
use tokio_tungstenite::tungstenite;

#[derive(Debug, Error)]
pub enum VenueError {
    #[error("bad venue message: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Core(#[from] CarryError),

    #[error("websocket: {0}")]
    WebSocket(Box<tungstenite::Error>),

    #[error("connection closed by venue")]
    Closed,

    #[error("event channel full: consumer too slow")]
    Overflow,

    #[error("resync requested")]
    ResyncRequested,

    #[error("http: {0}")]
    Http(#[from] reqwest::Error),

    #[error("market {0} not found in venue response")]
    MissingMarket(String),

    #[error("not a decimal number: {0}")]
    BadNumber(String),
}

impl From<tungstenite::Error> for VenueError {
    fn from(err: tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(err))
    }
}
