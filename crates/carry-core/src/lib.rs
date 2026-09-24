//! Core domain types, order book, funding models and slippage walk. No async, no I/O.

mod book;
mod error;
mod funding;
mod price;
mod qty;
mod side;
mod tick;
mod venue;

pub use book::{Fill, Level, LevelUpdate, OrderBook};
pub use error::CarryError;
pub use funding::{LighterFundingParams, lighter_hourly_funding};
pub use price::Price;
pub use qty::Qty;
pub use side::Side;
pub use tick::Tick;
pub use venue::Venue;
