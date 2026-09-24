//! Core domain types, order book, funding models and slippage walk. No async, no I/O.

mod error;
mod price;
mod qty;
mod tick;

pub use error::CarryError;
pub use price::Price;
pub use qty::Qty;
pub use tick::Tick;
