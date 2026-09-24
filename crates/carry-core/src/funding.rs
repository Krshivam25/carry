use crate::CarryError;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LighterFundingParams {
    premium_multiplier: Decimal,
    interest_rate: Decimal,
    small_clamp: Decimal,
    big_clamp: Decimal,
}

impl LighterFundingParams {
    pub fn new(
        premium_multiplier: Decimal,
        interest_rate: Decimal,
        small_clamp: Decimal,
        big_clamp: Decimal,
    ) -> Result<Self, CarryError> {
        for (name, value) in [
            ("premium_multiplier", premium_multiplier),
            ("small_clamp", small_clamp),
            ("big_clamp", big_clamp),
        ] {
            if value.is_sign_negative() {
                return Err(CarryError::InvalidFundingParam { name, value });
            }
        }
        Ok(Self {
            premium_multiplier,
            interest_rate,
            small_clamp,
            big_clamp,
        })
    }

    pub fn crypto() -> Self {
        Self {
            premium_multiplier: Decimal::ONE,
            interest_rate: Decimal::new(1, 4),
            small_clamp: Decimal::new(5, 4),
            big_clamp: Decimal::new(4, 2),
        }
    }
}

pub fn lighter_hourly_funding(
    avg_premium: Decimal,
    params: &LighterFundingParams,
) -> Result<Decimal, CarryError> {
    let premium = avg_premium
        .checked_mul(params.premium_multiplier)
        .ok_or(CarryError::Overflow)?;
    let small = params
        .small_clamp
        .checked_mul(params.premium_multiplier)
        .ok_or(CarryError::Overflow)?;
    let interest_adj = params
        .interest_rate
        .checked_sub(premium)
        .ok_or(CarryError::Overflow)?
        .clamp(-small, small);
    let small_clamped = premium
        .checked_add(interest_adj)
        .ok_or(CarryError::Overflow)?;
    let big_clamped = small_clamped.clamp(-params.big_clamp, params.big_clamp);
    Ok(big_clamped / Decimal::from(8))
}
