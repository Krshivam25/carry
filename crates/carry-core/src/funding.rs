use crate::CarryError;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LighterFundingParams {
    premium_multiplier: Decimal,
    interest_rate: Decimal,
    small_clamp: Decimal,
    big_clamp: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HourlyRate(Decimal);

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

impl HourlyRate {
    pub fn new(rate: Decimal) -> Self {
        Self(rate)
    }
    pub fn from_8h(rate_8h: Decimal) -> Result<Self, CarryError> {
        rate_8h
            .checked_div(Decimal::from(8))
            .map(Self)
            .ok_or(CarryError::Overflow)
    }
    pub fn value(self) -> Decimal {
        self.0
    }
    pub fn apr(self) -> Result<Decimal, CarryError> {
        self.0
            .checked_mul(Decimal::from(8760))
            .ok_or(CarryError::Overflow)
    }
}

pub trait FundingModel {
    fn hourly_rate(&self, avg_premium: Decimal) -> Result<HourlyRate, CarryError>;
}

impl FundingModel for LighterFundingParams {
    fn hourly_rate(&self, avg_premium: Decimal) -> Result<HourlyRate, CarryError> {
        lighter_hourly_funding(avg_premium, self).map(HourlyRate::new)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HyperliquidFundingParams {
    interest_8h: Decimal,
    clamp: Decimal,
    hourly_cap: Decimal,
}

impl HyperliquidFundingParams {
    pub fn new(
        interest_8h: Decimal,
        clamp: Decimal,
        hourly_cap: Decimal,
    ) -> Result<Self, CarryError> {
        for (name, value) in [("clamp", clamp), ("hourly_cap", hourly_cap)] {
            if value.is_sign_negative() {
                return Err(CarryError::InvalidFundingParam { name, value });
            }
        }
        Ok(Self {
            interest_8h,
            clamp,
            hourly_cap,
        })
    }

    pub fn crypto() -> Self {
        Self {
            interest_8h: Decimal::new(1, 4),
            clamp: Decimal::new(5, 4),
            hourly_cap: Decimal::new(4, 2),
        }
    }
}

impl FundingModel for HyperliquidFundingParams {
    fn hourly_rate(&self, avg_premium: Decimal) -> Result<HourlyRate, CarryError> {
        let interest_adj = self
            .interest_8h
            .checked_sub(avg_premium)
            .ok_or(CarryError::Overflow)?
            .clamp(-self.clamp, self.clamp);

        let rate_8h = avg_premium
            .checked_add(interest_adj)
            .ok_or(CarryError::Overflow)?;

        let hourly = HourlyRate::from_8h(rate_8h)?.value();
        Ok(HourlyRate::new(
            hourly.clamp(-self.hourly_cap, self.hourly_cap),
        ))
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
