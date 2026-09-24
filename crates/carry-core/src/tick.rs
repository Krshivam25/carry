use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::{CarryError, Price};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(u64);

impl Tick {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub const fn index(self) -> u64 {
        self.0
    }

    pub fn from_price(price: Price, tick_size: Decimal) -> Result<Self, CarryError> {
        if tick_size <= Decimal::ZERO {
            return Err(CarryError::InvalidTickSize(tick_size));
        }
        let steps = price
            .value()
            .checked_div(tick_size)
            .ok_or(CarryError::Overflow)?;
        if !steps.fract().is_zero() {
            return Err(CarryError::OffTick {
                price: price.value(),
                tick_size,
            });
        }
        steps.to_u64().map(Self).ok_or(CarryError::Overflow)
    }

    pub fn to_price(self, tick_size: Decimal) -> Result<Price, CarryError> {
        if tick_size <= Decimal::ZERO {
            return Err(CarryError::InvalidTickSize(tick_size));
        }
        let value = Decimal::from(self.0)
            .checked_mul(tick_size)
            .ok_or(CarryError::Overflow)?;
        Price::new(value)
    }
}
