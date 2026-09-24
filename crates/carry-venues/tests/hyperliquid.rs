use carry_core::{CarryError, Level, OrderBook, Qty, Tick};
use carry_venues::VenueError;
use carry_venues::hyperliquid::HlMessage;
use rust_decimal::Decimal;

const L2BOOK: &str = include_str!("fixtures/hl_l2book.json");

fn tick_size() -> Decimal {
    Decimal::new(1, 1)
}

fn level(tick: u64, mantissa: i64, scale: u32) -> Result<Level, CarryError> {
    Ok(Level {
        tick: Tick::new(tick),
        qty: Qty::new(Decimal::new(mantissa, scale))?,
    })
}

#[test]
fn parses_l2book() -> Result<(), VenueError> {
    let HlMessage::L2Book(book) = HlMessage::parse(L2BOOK)? else {
        panic!("expected L2Book");
    };
    assert_eq!(book.coin, "BTC");
    assert_eq!(book.levels[0].len(), 2);
    assert_eq!(book.levels[0][0].px, Decimal::new(654321, 1));
    assert_eq!(book.levels[0][0].n, 3);
    Ok(())
}

#[test]
fn l2book_feeds_order_book() -> Result<(), VenueError> {
    let HlMessage::L2Book(msg) = HlMessage::parse(L2BOOK)? else {
        panic!("expected L2Book");
    };
    let (bids, asks) = msg.to_levels(tick_size())?;
    let mut book = OrderBook::new(tick_size())?;
    book.apply_snapshot(msg.time, &bids, &asks)?;
    assert_eq!(book.best_bid(), Some(level(654_321, 5, 1)?));
    assert_eq!(book.best_ask(), Some(level(654_322, 75, 2)?));
    Ok(())
}

#[test]
fn subscription_ack_and_unknown_channels_parse() -> Result<(), VenueError> {
    let ack = r#"{"channel":"subscriptionResponse","data":{"method":"subscribe","subscription":{"type":"l2Book","coin":"BTC"}}}"#;
    assert!(matches!(
        HlMessage::parse(ack)?,
        HlMessage::SubscriptionResponse
    ));
    assert!(matches!(
        HlMessage::parse(r#"{"channel":"pong"}"#)?,
        HlMessage::Pong
    ));
    assert!(matches!(
        HlMessage::parse(r#"{"channel":"trades","data":[{"px":"1"}]}"#)?,
        HlMessage::Other(channel) if channel == "trades"
    ));
    Ok(())
}

#[test]
fn off_tick_price_is_a_core_error() -> Result<(), VenueError> {
    let HlMessage::L2Book(msg) = HlMessage::parse(L2BOOK)? else {
        panic!("expected L2Book");
    };
    // Tick size 1 cannot represent 65432.1.
    let result = msg.to_levels(Decimal::ONE);
    assert!(matches!(
        result,
        Err(VenueError::Core(CarryError::OffTick { .. }))
    ));
    Ok(())
}

#[test]
fn numeric_price_is_rejected() {
    let bad = r#"{"channel":"l2Book","data":{"coin":"BTC","time":1,"levels":[[{"px":65432.1,"sz":"1","n":1}],[]]}}"#;
    assert!(matches!(HlMessage::parse(bad), Err(VenueError::Json(_))));
}
