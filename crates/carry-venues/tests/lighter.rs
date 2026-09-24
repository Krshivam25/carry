use carry_core::{CarryError, Level, OrderBook, Qty, Tick};
use carry_venues::VenueError;
use carry_venues::lighter::{LighterBook, LighterMessage};
use rust_decimal::Decimal;

const SNAPSHOT: &str = include_str!("fixtures/lighter_snapshot.json");
const UPDATE: &str = include_str!("fixtures/lighter_update.json");

fn tick_size() -> Decimal {
    Decimal::new(1, 1)
}

fn level(tick: u64, mantissa: i64, scale: u32) -> Result<Level, CarryError> {
    Ok(Level {
        tick: Tick::new(tick),
        qty: Qty::new(Decimal::new(mantissa, scale))?,
    })
}

fn snapshot() -> Result<LighterBook, VenueError> {
    match serde_json::from_str(SNAPSHOT)? {
        LighterMessage::Snapshot(msg) => Ok(msg.order_book),
        other => panic!("expected Snapshot, got {other:?}"),
    }
}

fn update() -> Result<LighterBook, VenueError> {
    match serde_json::from_str(UPDATE)? {
        LighterMessage::Update(msg) => Ok(msg.order_book),
        other => panic!("expected Update, got {other:?}"),
    }
}

fn synced_book() -> Result<OrderBook, VenueError> {
    let snap = snapshot()?;
    let (bids, asks) = snap.to_levels(tick_size())?;
    let mut book = OrderBook::new(tick_size())?;
    book.apply_snapshot(snap.nonce, &bids, &asks)?;
    Ok(book)
}

#[test]
fn snapshot_feeds_order_book() -> Result<(), VenueError> {
    let book = synced_book()?;
    assert_eq!(book.seq(), Some(5000));
    assert_eq!(book.best_bid(), Some(level(654_321, 5, 1)?));
    assert_eq!(book.best_ask(), Some(level(654_322, 75, 2)?));
    Ok(())
}

#[test]
fn update_applies_when_begin_nonce_matches() -> Result<(), VenueError> {
    let mut book = synced_book()?;
    let upd = update()?;
    book.apply_delta_from(upd.begin_nonce, upd.nonce, &upd.to_updates(tick_size())?)?;
    assert_eq!(book.seq(), Some(5007));
    assert_eq!(book.best_bid(), Some(level(654_321, 9, 1)?)); // resized
    assert_eq!(book.best_ask(), Some(level(654_325, 2, 0)?)); // 65432.2 removed
    Ok(())
}

#[test]
fn update_with_wrong_begin_nonce_is_a_gap() -> Result<(), VenueError> {
    let mut book = synced_book()?;
    let upd = update()?;
    let result = book.apply_delta_from(upd.begin_nonce + 1, upd.nonce, &[]);
    assert_eq!(
        result,
        Err(CarryError::SequenceGap {
            expected: 5000,
            got: 5001
        })
    );
    assert!(!book.is_synced());
    Ok(())
}

#[test]
fn pong_and_unknown_types_parse() -> Result<(), VenueError> {
    assert!(matches!(
        serde_json::from_str(r#"{"type":"pong"}"#)?,
        LighterMessage::Pong
    ));
    assert!(matches!(
        serde_json::from_str(r#"{"type":"connected","session_id":"abc"}"#)?,
        LighterMessage::Other
    ));
    Ok(())
}
