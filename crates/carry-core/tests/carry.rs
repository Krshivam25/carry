use carry_core::{CarryError, HourlyRate, Leg, Level, OrderBook, Qty, Tick, executable_carry};
use rust_decimal::Decimal;

fn level(tick: u64, size: i64) -> Result<Level, CarryError> {
    Ok(Level {
        tick: Tick::new(tick),
        qty: Qty::new(Decimal::new(size, 0))?,
    })
}

fn book(bid: u64, ask: u64) -> Result<OrderBook, CarryError> {
    let mut book = OrderBook::new(Decimal::ONE)?;
    book.apply_snapshot(1, &[level(bid, 100)?], &[level(ask, 100)?])?;
    Ok(book)
}

fn usd(n: i64) -> Decimal {
    Decimal::new(n, 0)
}

#[test]
fn mid_is_average_of_best_prices() -> Result<(), CarryError> {
    assert_eq!(book(75, 125)?.mid()?, Some(usd(100)));
    let mut one_sided = OrderBook::new(Decimal::ONE)?;
    one_sided.apply_snapshot(1, &[level(75, 1)?], &[])?;
    assert_eq!(one_sided.mid()?, None);
    Ok(())
}

#[test]
fn carry_after_costs() -> Result<(), CarryError> {
    let long_book = book(75, 125)?;
    let short_book = book(80, 120)?;
    let long = Leg {
        book: &long_book,
        funding: HourlyRate::new(Decimal::new(-1, 5)),
        taker_fee: Decimal::new(45, 5),
    };
    let short = Leg {
        book: &short_book,
        funding: HourlyRate::new(Decimal::new(3, 5)),
        taker_fee: Decimal::ZERO,
    };
    let carry = executable_carry(&long, &short, usd(1000))?;
    assert_eq!(carry.hourly_spread, HourlyRate::new(Decimal::new(4, 5)));
    assert_eq!(carry.carry_per_hour, Decimal::new(4, 2));
    assert_eq!(carry.entry_cost, Decimal::new(45_045, 2));
    assert_eq!(carry.round_trip_cost, Decimal::new(90_090, 2));
    assert_eq!(carry.breakeven_hours, Some(Decimal::new(225_225, 1)));
    assert_eq!(carry.net_carry(usd(168))?, Decimal::new(-89_418, 2));
    Ok(())
}

#[test]
fn negative_spread_never_breaks_even() -> Result<(), CarryError> {
    let a = book(75, 125)?;
    let b = book(80, 120)?;
    let long = Leg {
        book: &a,
        funding: HourlyRate::new(Decimal::new(3, 5)),
        taker_fee: Decimal::ZERO,
    };
    let short = Leg {
        book: &b,
        funding: HourlyRate::new(Decimal::new(-1, 5)),
        taker_fee: Decimal::ZERO,
    };
    let carry = executable_carry(&long, &short, usd(1000))?;
    assert!(carry.carry_per_hour < Decimal::ZERO);
    assert_eq!(carry.breakeven_hours, None);
    Ok(())
}

#[test]

fn stale_book_is_refused() -> Result<(), CarryError> {
    let fresh = book(75, 125)?;
    let mut stale = book(80, 120)?;
    stale.mark_stale();
    let long = Leg {
        book: &fresh,
        funding: HourlyRate::new(Decimal::ZERO),
        taker_fee: Decimal::ZERO,
    };
    let short = Leg {
        book: &stale,
        funding: HourlyRate::new(Decimal::ZERO),
        taker_fee: Decimal::ZERO,
    };
    assert_eq!(
        executable_carry(&long, &short, usd(1000)),
        Err(CarryError::NotSynced)
    );
    Ok(())
}
