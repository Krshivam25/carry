use carry_venues::{BookEvent, VenueError, hyperliquid, lighter};
use rust_decimal::Decimal;

const HL_L2BOOK: &str = include_str!("fixtures/hl_l2book.json");
const LIGHTER_SNAPSHOT: &str = include_str!("fixtures/lighter_snapshot.json");
const LIGHTER_UPDATE: &str = include_str!("fixtures/lighter_update.json");

fn tick() -> Decimal {
    Decimal::new(1, 1)
}

#[test]
fn hl_book_is_a_snapshot_event() -> Result<(), VenueError> {
    let Some(BookEvent::Snapshot { seq, bids, asks }) = hyperliquid::to_event(HL_L2BOOK, tick())?
    else {
        panic!("expected Snapshot");
    };
    assert_eq!(seq, 1_758_790_000_000);
    assert_eq!((bids.len(), asks.len()), (2, 2));
    Ok(())
}

#[test]
fn hl_pong_is_no_event() -> Result<(), VenueError> {
    assert_eq!(
        hyperliquid::to_event(r#"{"channel":"pong"}"#, tick())?,
        None
    );
    Ok(())
}

#[test]
fn lighter_snapshot_and_update_events() -> Result<(), VenueError> {
    let Some(BookEvent::Snapshot { seq, .. }) = lighter::to_event(LIGHTER_SNAPSHOT, tick())? else {
        panic!("expected Snapshot");
    };
    assert_eq!(seq, 5000);

    let Some(BookEvent::Delta {
        prev_seq,
        seq,
        updates,
    }) = lighter::to_event(LIGHTER_UPDATE, tick())?
    else {
        panic!("expected Delta");
    };
    assert_eq!((prev_seq, seq, updates.len()), (5000, 5007, 2));
    Ok(())
}

#[test]
fn lighter_pong_is_no_event() -> Result<(), VenueError> {
    assert_eq!(lighter::to_event(r#"{"type":"pong"}"#, tick())?, None);
    Ok(())
}
