use crate::Tick;
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

    #[error("sequence gap: expected {expected}, got {got}")]
    SequenceGap { expected: u64, got: u64 },

    #[error("book is not synced: waiting for snapshot")]
    NotSynced,

    #[error("crossed book: best bid {bid:?} >= best ask {ask:?}")]
    CrossedBook { bid: Tick, ask: Tick },

    #[error("invalid notional {0}: must be > 0")]
    InvalidNotional(Decimal),

    #[error("insufficient liquidity: requested {requested}, available {available}")]
    InsufficientLiquidity {
        requested: Decimal,
        available: Decimal,
    },
}
