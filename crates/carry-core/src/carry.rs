use rust_decimal::Decimal;

use crate::{CarryError, HourlyRate, OrderBook, Side};

#[derive(Debug, Clone, Copy)]
pub struct Leg<'a> {
    pub book: &'a OrderBook,
    pub funding: HourlyRate,
    pub taker_fee: Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutableCarry {
    pub notional: Decimal,
    pub hourly_spread: HourlyRate,
    pub carry_per_hour: Decimal,
    pub entry_cost: Decimal,
    pub round_trip_cost: Decimal,
    pub breakeven_hours: Option<Decimal>,
}

impl ExecutableCarry {
    pub fn net_carry(&self, hours: Decimal) -> Result<Decimal, CarryError> {
        self.carry_per_hour
            .checked_mul(hours)
            .and_then(|gross| gross.checked_sub(self.round_trip_cost))
            .ok_or(CarryError::Overflow)
    }
}

pub fn executable_carry(
    long: &Leg<'_>,
    short: &Leg<'_>,
    notional: Decimal,
) -> Result<ExecutableCarry, CarryError> {
    let long_cost = leg_cost(long, Side::Ask, notional)?;
    let short_cost = leg_cost(short, Side::Bid, notional)?;
    let entry_cost = long_cost
        .checked_add(short_cost)
        .ok_or(CarryError::Overflow)?;
    let round_trip_cost = entry_cost
        .checked_mul(Decimal::TWO)
        .ok_or(CarryError::Overflow)?;

    let spread = short
        .funding
        .value()
        .checked_sub(long.funding.value())
        .ok_or(CarryError::Overflow)?;
    let carry_per_hour = notional.checked_mul(spread).ok_or(CarryError::Overflow)?;
    let breakeven_hours = if carry_per_hour > Decimal::ZERO {
        Some(
            round_trip_cost
                .checked_div(carry_per_hour)
                .ok_or(CarryError::Overflow)?,
        )
    } else {
        None
    };

    Ok(ExecutableCarry {
        notional,
        hourly_spread: HourlyRate::new(spread),
        carry_per_hour,
        entry_cost,
        round_trip_cost,
        breakeven_hours,
    })
}

fn leg_cost(leg: &Leg<'_>, side: Side, notional: Decimal) -> Result<Decimal, CarryError> {
    let fill = leg.book.walk(side, notional)?;
    let mid = leg.book.mid()?.ok_or(CarryError::OneSidedBook)?;
    let worse_by = match side {
        Side::Ask => fill.avg_price - mid,
        Side::Bid => mid - fill.avg_price,
    };
    let slippage = notional
        .checked_mul(worse_by)
        .and_then(|x| x.checked_div(mid))
        .ok_or(CarryError::Overflow)?;
    let fee = notional
        .checked_mul(leg.taker_fee)
        .ok_or(CarryError::Overflow)?;
    slippage.checked_add(fee).ok_or(CarryError::Overflow)
}
