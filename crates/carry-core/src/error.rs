use rust_decimal::Decimal;
use thiserror::Error;

/// Errors produced by carry-core.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CarryError {
    /// A price was zero or negative.
    #[error("invalid price {0}: must be > 0")]
    InvalidPrice(Decimal),
}
