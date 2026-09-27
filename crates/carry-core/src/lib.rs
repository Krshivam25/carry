//! Core domain types, order book, funding models and slippage walk. No async, no I/O.

mod book;
mod carry;
mod error;
mod funding;
mod price;
mod qty;
mod side;
mod tick;
mod venue;

pub use book::{Fill, Level, LevelUpdate, OrderBook};
pub use carry::{ExecutableCarry, Leg, executable_carry};
pub use error::CarryError;
pub use funding::{
    FundingModel, HourlyRate, HyperliquidFundingParams, LighterFundingParams,
    lighter_hourly_funding,
};
pub use price::Price;
pub use qty::Qty;
pub use side::Side;
pub use tick::Tick;
pub use venue::Venue;
