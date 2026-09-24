use rust_decimal::Decimal;

use crate::CarryError;

/// A strictly positive price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(Decimal);

impl Price {
    /// Creates a price, rejecting zero and negative values.
    pub fn new(value: Decimal) -> Result<Self, CarryError> {
        if value <= Decimal::ZERO {
            return Err(CarryError::InvalidPrice(value));
        }
        Ok(Self(value))
    }

    /// Returns the underlying decimal value.
    pub fn value(self) -> Decimal {
        self.0
    }
}
