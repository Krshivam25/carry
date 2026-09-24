use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CarryError {
    #[error("invalid price {0}: must be > 0")]
    InvalidPrice(Decimal),

    #[error("invalid quantity {0}: must be >= 0")]
    InvalidQty(Decimal),

    #[error("invalid tick size {0}: must be > 0")]
    InvalidTickSize(Decimal),

    #[error("price {price} is not a multiple of tick size {tick_size}")]
    OffTick { price: Decimal, tick_size: Decimal },

    #[error("arithmetic overflow")]
    Overflow,

    #[error("unknown venue {0:?}")]
    UnknownVenue(String),

    #[error("invalid funding parameter {name}: {value}")]
    InvalidFundingParam { name: &'static str, value: Decimal },
}
