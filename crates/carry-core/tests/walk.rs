use carry_core::{CarryError, Fill, Level, OrderBook, Qty, Side, Tick};
use rust_decimal::Decimal;

fn level(tick: u64, size: i64) -> Result<Level, CarryError> {
    Ok(Level {
        tick: Tick::new(tick),
        qty: Qty::new(Decimal::new(size, 0))?,
    })
}

fn usd(n: i64) -> Decimal {
    Decimal::new(n, 0)
}

fn book() -> Result<OrderBook, CarryError> {
    let mut book = OrderBook::new(Decimal::ONE)?;
    book.apply_snapshot(
        1,
        &[level(80, 1)?, level(60, 5)?],
        &[level(100, 1)?, level(105, 4)?, level(110, 10)?],
    )?;
    Ok(book)
}

#[test]
fn buy_inside_top_level_has_no_slippage() -> Result<(), CarryError> {
    let fill = book()?.walk(Side::Ask, usd(50))?;
    assert_eq!(
        fill,
        Fill {
            base_qty: Decimal::new(5, 1), // 0.5
            notional: usd(50),
            avg_price: usd(100),
            best_price: usd(100),
            worst_price: usd(100),
            levels_used: 1,
            slippage_bps: Decimal::ZERO,
        }
    );
    Ok(())
}

#[test]
fn buy_exactly_top_level_stops_at_boundary() -> Result<(), CarryError> {
    let fill = book()?.walk(Side::Ask, usd(100))?;
    assert_eq!(fill.base_qty, Decimal::ONE);
    assert_eq!(fill.levels_used, 1);
    Ok(())
}

#[test]
fn buy_across_two_levels() -> Result<(), CarryError> {
    // $100 at 100 (1 unit) + $105 at 105 (1 unit) = 2 units, avg 102.5, 250 bps
    let fill = book()?.walk(Side::Ask, usd(205))?;
    assert_eq!(fill.base_qty, usd(2));
    assert_eq!(fill.avg_price, Decimal::new(1025, 1));
    assert_eq!(fill.worst_price, usd(105));
    assert_eq!(fill.levels_used, 2);
    assert_eq!(fill.slippage_bps, usd(250));
    Ok(())
}

#[test]
fn sell_walks_bids_from_highest() -> Result<(), CarryError> {
    let fill = book()?.walk(Side::Bid, usd(140))?;
    assert_eq!(fill.base_qty, usd(2));
    assert_eq!(fill.avg_price, usd(70));
    assert_eq!(fill.best_price, usd(80));
    assert_eq!(fill.worst_price, usd(60));
    assert_eq!(fill.slippage_bps, usd(1250));
    Ok(())
}

#[test]
fn too_large_is_insufficient_liquidity() -> Result<(), CarryError> {
    assert_eq!(
        book()?.walk(Side::Ask, usd(2000)),
        Err(CarryError::InsufficientLiquidity {
            requested: usd(2000),
            available: usd(1620),
        })
    );
    Ok(())
}

#[test]
fn unsynced_book_refuses_to_walk() -> Result<(), CarryError> {
    let book = OrderBook::new(Decimal::ONE)?;
    assert_eq!(book.walk(Side::Ask, usd(10)), Err(CarryError::NotSynced));
    Ok(())
}

#[test]
fn zero_notional_is_rejected() -> Result<(), CarryError> {
    assert_eq!(
        book()?.walk(Side::Ask, Decimal::ZERO),
        Err(CarryError::InvalidNotional(Decimal::ZERO))
    );
    Ok(())
}

#[test]
fn bigger_order_never_gets_better_price() -> Result<(), CarryError> {
    let book = book()?;
    let small = book.walk(Side::Ask, usd(50))?;
    let large = book.walk(Side::Ask, usd(205))?;
    assert!(large.avg_price >= small.avg_price);
    Ok(())
}
