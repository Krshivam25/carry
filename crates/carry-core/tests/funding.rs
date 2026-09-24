use carry_core::{CarryError, LighterFundingParams, lighter_hourly_funding};
use rust_decimal::Decimal;

fn crypto(avg_premium: Decimal) -> Result<Decimal, CarryError> {
    lighter_hourly_funding(avg_premium, &LighterFundingParams::crypto())
}

#[test]
fn zero_premium_pays_interest_only() -> Result<(), CarryError> {
    assert_eq!(crypto(Decimal::ZERO)?, Decimal::new(125, 7));
    Ok(())
}

#[test]
fn small_premium_is_pulled_to_interest_rate() -> Result<(), CarryError> {
    assert_eq!(crypto(Decimal::new(3, 4))?, Decimal::new(125, 7));
    Ok(())
}

#[test]
fn large_premium_only_shifted_by_small_clamp() -> Result<(), CarryError> {
    assert_eq!(crypto(Decimal::new(1, 2))?, Decimal::new(11875, 7));
    Ok(())
}

#[test]
fn caps_at_plus_half_percent_per_hour() -> Result<(), CarryError> {
    assert_eq!(crypto(Decimal::ONE)?, Decimal::new(5, 3));
    Ok(())
}

#[test]
fn caps_at_minus_half_percent_per_hour() -> Result<(), CarryError> {
    assert_eq!(crypto(Decimal::NEGATIVE_ONE)?, Decimal::new(-5, 3));
    Ok(())
}

#[test]
fn premium_multiplier_scales_premium_and_small_clamp() -> Result<(), CarryError> {
    let rwa = LighterFundingParams::new(
        Decimal::new(5, 1),
        Decimal::new(1, 4),
        Decimal::new(5, 4),
        Decimal::new(4, 2),
    )?;
    assert_eq!(
        lighter_hourly_funding(Decimal::new(1, 2), &rwa)?,
        Decimal::new(59375, 8)
    );
    Ok(())
}

#[test]
fn params_reject_negative_clamp() {
    let bad = Decimal::new(-1, 4);
    assert_eq!(
        LighterFundingParams::new(Decimal::ONE, Decimal::new(1, 4), bad, Decimal::new(4, 2)),
        Err(CarryError::InvalidFundingParam {
            name: "small_clamp",
            value: bad
        })
    );
}
