use carry_core::{CarryError, Venue};

#[test]
fn display_is_lowercase_name() {
    assert_eq!(Venue::Hyperliquid.to_string(), "hyperliquid");
    assert_eq!(Venue::Lighter.to_string(), "lighter");
}

#[test]
fn parse_round_trips_display() -> Result<(), CarryError> {
    for venue in Venue::ALL {
        assert_eq!(venue.to_string().parse::<Venue>()?, venue);
    }
    Ok(())
}

#[test]
fn parse_is_case_insensitive_and_accepts_alias() -> Result<(), CarryError> {
    assert_eq!("HyperLiquid".parse::<Venue>()?, Venue::Hyperliquid);
    assert_eq!("hl".parse::<Venue>()?, Venue::Hyperliquid);
    assert_eq!("LIGHTER".parse::<Venue>()?, Venue::Lighter);
    Ok(())
}

#[test]
fn parse_rejects_unknown() {
    assert_eq!(
        "binance".parse::<Venue>(),
        Err(CarryError::UnknownVenue("binance".to_owned()))
    );
}
