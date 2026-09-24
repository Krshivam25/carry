use carry_core::{CarryError, Price, Tick};
use rust_decimal::Decimal;

fn price(mantissa: i64, scale: u32) -> Result<Price, CarryError> {
    Price::new(Decimal::new(mantissa, scale))
}

#[test]
fn from_price_on_tick() -> Result<(), CarryError> {
    let tick_size = Decimal::new(5, 1);
    let tick = Tick::from_price(price(1005, 1)?, tick_size)?;
    assert_eq!(tick.index(), 201);
    Ok(())
}

#[test]
fn round_trip_price_tick_price() -> Result<(), CarryError> {
    let tick_size = Decimal::new(1, 2);
    let p = price(6543210, 2)?;
    let tick = Tick::from_price(p, tick_size)?;
    assert_eq!(tick.to_price(tick_size)?, p);
    Ok(())
}

#[test]
fn from_price_rejects_off_tick() -> Result<(), CarryError> {
    let tick_size = Decimal::new(5, 1);
    let p = price(1003, 1)?;
    assert_eq!(
        Tick::from_price(p, tick_size),
        Err(CarryError::OffTick {
            price: p.value(),
            tick_size
        })
    );
    Ok(())
}

#[test]
fn rejects_non_positive_tick_size() -> Result<(), CarryError> {
    assert_eq!(
        Tick::from_price(price(100, 0)?, Decimal::ZERO),
        Err(CarryError::InvalidTickSize(Decimal::ZERO))
    );
    assert_eq!(
        Tick::new(1).to_price(Decimal::ZERO),
        Err(CarryError::InvalidTickSize(Decimal::ZERO))
    );
    Ok(())
}

#[test]
fn zero_tick_is_not_a_valid_price() {
    assert_eq!(
        Tick::new(0).to_price(Decimal::new(1, 2)),
        Err(CarryError::InvalidPrice(Decimal::ZERO))
    );
}

#[test]
fn ticks_order_like_prices() -> Result<(), CarryError> {
    let tick_size = Decimal::ONE;
    let low = Tick::from_price(price(100, 0)?, tick_size)?;
    let high = Tick::from_price(price(101, 0)?, tick_size)?;
    assert!(low < high);
    Ok(())
}
