use carry_core::{
    CarryError, FundingModel, HourlyRate, HyperliquidFundingParams, LighterFundingParams,
};
use rust_decimal::Decimal;

fn hl(avg_premium: Decimal) -> Result<Decimal, CarryError> {
    Ok(HyperliquidFundingParams::crypto()
        .hourly_rate(avg_premium)?
        .value())
}

#[test]
fn hl_matches_live_btc_funding() -> Result<(), CarryError> {
    assert_eq!(hl(Decimal::new(-2_862_909, 10))?, Decimal::new(125, 7));
    Ok(())
}

#[test]
fn hl_docs_example_one_percent_premium() -> Result<(), CarryError> {
    assert_eq!(hl(Decimal::new(1, 2))?, Decimal::new(11_875, 7));
    Ok(())
}

#[test]
fn hl_is_capped_at_four_percent_per_hour() -> Result<(), CarryError> {
    assert_eq!(hl(Decimal::ONE)?, Decimal::new(4, 2));
    assert_eq!(hl(Decimal::NEGATIVE_ONE)?, Decimal::new(-4, 2));
    Ok(())
}

#[test]
fn hourly_rate_conversions() -> Result<(), CarryError> {
    let rate = HourlyRate::from_8h(Decimal::new(1, 4))?;
    assert_eq!(rate.value(), Decimal::new(125, 7));
    assert_eq!(rate.apr()?, Decimal::new(1095, 4));
    Ok(())
}

#[test]
fn venues_differ_only_in_their_caps() -> Result<(), CarryError> {
    let hl = HyperliquidFundingParams::crypto();
    let lighter = LighterFundingParams::crypto();
    let models: [&dyn FundingModel; 2] = [&hl, &lighter];

    for model in models {
        assert_eq!(
            model.hourly_rate(Decimal::new(1, 2))?.value(),
            Decimal::new(11_875, 7)
        );
    }

    let capped: Vec<Decimal> = models
        .iter()
        .map(|m| m.hourly_rate(Decimal::ONE).map(HourlyRate::value))
        .collect::<Result<_, _>>()?;
    assert_eq!(capped, [Decimal::new(4, 2), Decimal::new(5, 3)]);
    Ok(())
}

#[test]
fn hl_params_reject_negative_cap() {
    let bad = Decimal::NEGATIVE_ONE;
    assert_eq!(
        HyperliquidFundingParams::new(Decimal::new(1, 4), Decimal::new(5, 4), bad),
        Err(CarryError::InvalidFundingParam {
            name: "hourly_cap",
            value: bad
        })
    );
}
