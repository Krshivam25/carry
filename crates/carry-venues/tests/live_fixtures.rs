use carry_core::OrderBook;
use carry_venues::{BookEvent, VenueError, hyperliquid, lighter};
use rust_decimal::Decimal;

fn tick() -> Decimal {
    Decimal::new(1, 1)
}
fn apply(book: &mut OrderBook, event: Option<BookEvent>) -> Result<(), VenueError> {
    match event {
        Some(BookEvent::Snapshot { seq, bids, asks }) => book.apply_snapshot(seq, &bids, &asks)?,
        Some(BookEvent::Delta {
            prev_seq,
            seq,
            updates,
        }) => book.apply_delta_from(prev_seq, seq, &updates)?,
        other => panic!("expected a book event, got {other:?}"),
    }
    Ok(())
}

#[test]
fn live_hyperliquid_frames() -> Result<(), VenueError> {
    let ack = include_str!("fixtures/live_hl_ack.json");
    assert_eq!(hyperliquid::to_event(ack, tick())?, None);

    let mut book = OrderBook::new(tick())?;
    let event = hyperliquid::to_event(include_str!("fixtures/live_hl_l2book.json"), tick())?;
    apply(&mut book, event)?;
    assert!(book.is_synced());
    Ok(())
}

#[test]
fn live_lighter_snapshot_then_update_is_continuous() -> Result<(), VenueError> {
    let connected = include_str!("fixtures/live_lighter_connected.json");
    assert_eq!(lighter::to_event(connected, tick())?, None);

    let mut book = OrderBook::new(tick())?;
    let snapshot = lighter::to_event(include_str!("fixtures/live_lighter_snapshot.json"), tick())?;
    apply(&mut book, snapshot)?;
    let update = lighter::to_event(include_str!("fixtures/live_lighter_update.json"), tick())?;
    apply(&mut book, update)?;
    assert!(book.is_synced());
    assert_eq!(book.seq(), Some(22_900_816_362));
    Ok(())
}
