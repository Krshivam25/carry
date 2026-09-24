//! Core domain types, order book, funding models and slippage walk. No async, no I/O.

mod error;
mod price;

pub use error::CarryError;
pub use price::Price;
