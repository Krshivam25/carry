use rust_decimal::Decimal;

use crate::CarryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Qty(Decimal);

impl Qty {
    pub fn new(value: Decimal) -> Result<Self, CarryError> {
        if value.is_sign_negative() {
            return Err(CarryError::InvalidQty(value));
        }
        Ok(Self(value))
    }

    pub fn value(self) -> Decimal {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }
}
