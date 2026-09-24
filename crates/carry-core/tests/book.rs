use carry_core::{CarryError, Level, LevelUpdate, OrderBook, Qty, Side, Tick};
use rust_decimal::Decimal;

fn qty(n: i64) -> Result<Qty, CarryError> {
    Qty::new(Decimal::new(n, 0))
}

fn level(tick: u64, size: i64) -> Result<Level, CarryError> {
    Ok(Level {
        tick: Tick::new(tick),
        qty: qty(size)?,
    })
}

fn update(side: Side, tick: u64, size: i64) -> Result<LevelUpdate, CarryError> {
    Ok(LevelUpdate {
        side,
        tick: Tick::new(tick),
        qty: qty(size)?,
    })
}

fn synced_book() -> Result<OrderBook, CarryError> {
    let mut book = OrderBook::new(Decimal::ONE)?;
    book.apply_snapshot(
        10,
        &[level(99, 5)?, level(98, 7)?],
        &[level(101, 3)?, level(102, 4)?],
    )?;
    Ok(book)
}

#[test]
fn new_rejects_zero_tick_size() {
    assert_eq!(
        OrderBook::new(Decimal::ZERO).err(),
        Some(CarryError::InvalidTickSize(Decimal::ZERO))
    );
}

#[test]
fn new_book_is_empty_and_unsynced() -> Result<(), CarryError> {
    let book = OrderBook::new(Decimal::ONE)?;
    assert!(!book.is_synced());
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.best_ask(), None);
    Ok(())
}

#[test]
fn snapshot_sets_best_levels_and_skips_zero_qty() -> Result<(), CarryError> {
    let mut book = OrderBook::new(Decimal::ONE)?;
    book.apply_snapshot(10, &[level(99, 5)?, level(100, 0)?], &[level(101, 3)?])?;
    assert_eq!(book.seq(), Some(10));
    assert_eq!(book.best_bid(), Some(level(99, 5)?));
    assert_eq!(book.best_ask(), Some(level(101, 3)?));
    Ok(())
}

#[test]
fn delta_before_snapshot_is_not_synced() -> Result<(), CarryError> {
    let mut book = OrderBook::new(Decimal::ONE)?;
    assert_eq!(
        book.apply_delta(1, &[update(Side::Bid, 99, 1)?]),
        Err(CarryError::NotSynced)
    );
    Ok(())
}

#[test]
fn delta_updates_adds_and_removes_levels() -> Result<(), CarryError> {
    let mut book = synced_book()?;
    book.apply_delta(
        11,
        &[
            update(Side::Bid, 99, 0)?,  // remove best bid
            update(Side::Bid, 100, 2)?, // new best bid
            update(Side::Ask, 101, 9)?, // resize best ask
        ],
    )?;
    assert_eq!(book.seq(), Some(11));
    assert_eq!(book.best_bid(), Some(level(100, 2)?));
    assert_eq!(book.best_ask(), Some(level(101, 9)?));
    Ok(())
}

#[test]
fn removing_the_best_level_exposes_the_next() -> Result<(), CarryError> {
    let mut book = synced_book()?;
    book.apply_delta(11, &[update(Side::Bid, 99, 0)?])?;
    assert_eq!(book.best_bid(), Some(level(98, 7)?));
    Ok(())
}

#[test]
fn sequence_gap_errors_and_unsyncs() -> Result<(), CarryError> {
    let mut book = synced_book()?;
    assert_eq!(
        book.apply_delta(12, &[]),
        Err(CarryError::SequenceGap {
            expected: 11,
            got: 12
        })
    );
    assert!(!book.is_synced());
    assert_eq!(book.apply_delta(13, &[]), Err(CarryError::NotSynced));
    Ok(())
}

#[test]
fn duplicate_seq_is_a_gap() -> Result<(), CarryError> {
    let mut book = synced_book()?;
    assert_eq!(
        book.apply_delta(10, &[]),
        Err(CarryError::SequenceGap {
            expected: 11,
            got: 10
        })
    );
    Ok(())
}

#[test]
fn crossing_delta_errors_and_unsyncs() -> Result<(), CarryError> {
    let mut book = synced_book()?;
    assert_eq!(
        book.apply_delta(11, &[update(Side::Bid, 101, 1)?]),
        Err(CarryError::CrossedBook {
            bid: Tick::new(101),
            ask: Tick::new(101)
        })
    );
    assert!(!book.is_synced());
    Ok(())
}

#[test]
fn snapshot_after_gap_resyncs() -> Result<(), CarryError> {
    let mut book = synced_book()?;
    let _ = book.apply_delta(99, &[]); // gap -> unsynced
    book.apply_snapshot(50, &[level(90, 1)?], &[level(91, 1)?])?;
    assert_eq!(book.seq(), Some(50));
    book.apply_delta(51, &[update(Side::Ask, 92, 1)?])?;
    assert_eq!(book.seq(), Some(51));
    assert_eq!(book.best_bid(), Some(level(90, 1)?));
    Ok(())
}

#[test]
fn delta_from_advances_to_range_end() -> Result<(), CarryError> {
    let mut book = synced_book()?; // seq 10
    book.apply_delta_from(10, 17, &[update(Side::Bid, 100, 2)?])?;
    assert_eq!(book.seq(), Some(17));
    assert_eq!(book.best_bid(), Some(level(100, 2)?));
    Ok(())
}

#[test]
fn delta_from_with_wrong_begin_is_a_gap() -> Result<(), CarryError> {
    let mut book = synced_book()?;
    assert_eq!(
        book.apply_delta_from(12, 15, &[]),
        Err(CarryError::SequenceGap {
            expected: 10,
            got: 12
        })
    );
    assert!(!book.is_synced());
    Ok(())
}

#[test]
fn delta_from_backwards_range_is_rejected() -> Result<(), CarryError> {
    let mut book = synced_book()?;
    assert_eq!(
        book.apply_delta_from(10, 9, &[]),
        Err(CarryError::SequenceGap {
            expected: 10,
            got: 9
        })
    );
    assert!(!book.is_synced());
    Ok(())
}
