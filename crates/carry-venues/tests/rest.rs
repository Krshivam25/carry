use carry_core::HourlyRate;
use carry_venues::VenueError;
use carry_venues::rest::{parse_hl_funding, parse_lighter_funding, parse_lighter_taker_fee};
use rust_decimal::Decimal;

const HL_CTXS: &str = include_str!("fixtures/live_hl_meta_ctxs.json");
const LIGHTER_RATES: &str = include_str!("fixtures/live_lighter_funding_rates.json");
const LIGHTER_DETAILS: &str = include_str!("fixtures/live_lighter_order_book_details.json");

#[test]
fn hl_btc_funding_is_hourly() -> Result<(), VenueError> {
    assert_eq!(
        parse_hl_funding(HL_CTXS, "BTC")?,
        HourlyRate::new(Decimal::new(125, 7))
    );
    Ok(())
}

#[test]
fn hl_unknown_coin_is_missing() {
    assert!(matches!(
        parse_hl_funding(HL_CTXS, "NOPE"),
        Err(VenueError::MissingMarket(_))
    ));
}

#[test]
fn lighter_float_rate_is_parsed_without_f64() -> Result<(), VenueError> {
    assert_eq!(
        parse_lighter_funding(LIGHTER_RATES, 1)?,
        HourlyRate::new(Decimal::new(12, 6))
    );
    Ok(())
}

#[test]
fn lighter_ignores_other_exchanges_rows() -> Result<(), VenueError> {
    assert_ne!(
        parse_lighter_funding(LIGHTER_RATES, 1)?,
        HourlyRate::from_8h(Decimal::new(263, 7))?
    );
    Ok(())
}

#[test]
fn lighter_btc_taker_fee_is_zero() -> Result<(), VenueError> {
    assert_eq!(parse_lighter_taker_fee(LIGHTER_DETAILS, 1)?, Decimal::ZERO);
    assert!(matches!(
        parse_lighter_taker_fee(LIGHTER_DETAILS, 99),
        Err(VenueError::MissingMarket(_))
    ));
    Ok(())
}
