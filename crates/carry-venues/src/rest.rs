use std::str::FromStr;
use std::time::Duration;

use carry_core::HourlyRate;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use serde_json::value::RawValue;

use crate::VenueError;

const HL_INFO_URL: &str = "https://api.hyperliquid.xyz/info";
const LIGHTER_API: &str = "https://mainnet.zklighter.elliot.ai/api/v1";

pub const HL_BASE_TAKER_FEE: Decimal = Decimal::from_parts(45, 0, 0, false, 5);

const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

pub fn http_client() -> Result<reqwest::Client, VenueError> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    Ok(reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueQuote {
    pub funding: HourlyRate,
    pub taker_fee: Decimal,
}

pub async fn hyperliquid_quote(
    client: &reqwest::Client,
    coin: &str,
) -> Result<VenueQuote, VenueError> {
    let body = client
        .post(HL_INFO_URL)
        .json(&json!({ "type": "metaAndAssetCtxs" }))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Ok(VenueQuote {
        funding: parse_hl_funding(&body, coin)?,
        taker_fee: HL_BASE_TAKER_FEE,
    })
}

pub async fn lighter_quote(
    client: &reqwest::Client,
    market_id: u32,
) -> Result<VenueQuote, VenueError> {
    let rates = get_text(client, &format!("{LIGHTER_API}/funding-rates")).await?;
    let details = get_text(
        client,
        &format!("{LIGHTER_API}/orderBookDetails?market_id={market_id}"),
    )
    .await?;
    Ok(VenueQuote {
        funding: parse_lighter_funding(&rates, market_id)?,
        taker_fee: parse_lighter_taker_fee(&details, market_id)?,
    })
}

async fn get_text(client: &reqwest::Client, url: &str) -> Result<String, VenueError> {
    Ok(client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?)
}

#[derive(Deserialize)]
struct HlMeta {
    universe: Vec<HlAsset>,
}

#[derive(Deserialize)]
struct HlAsset {
    name: String,
}

#[derive(Deserialize)]
struct HlAssetCtx {
    #[serde(with = "rust_decimal::serde::str")]
    funding: Decimal,
}

pub fn parse_hl_funding(body: &str, coin: &str) -> Result<HourlyRate, VenueError> {
    let (meta, ctxs): (HlMeta, Vec<HlAssetCtx>) = serde_json::from_str(body)?;
    let index = meta
        .universe
        .iter()
        .position(|asset| asset.name == coin)
        .ok_or_else(|| VenueError::MissingMarket(coin.to_owned()))?;
    let ctx = ctxs
        .get(index)
        .ok_or_else(|| VenueError::MissingMarket(coin.to_owned()))?;
    Ok(HourlyRate::new(ctx.funding))
}

#[derive(Deserialize)]
struct LighterRates<'a> {
    #[serde(borrow)]
    funding_rates: Vec<LighterRate<'a>>,
}

#[derive(Deserialize)]
struct LighterRate<'a> {
    market_id: u32,
    exchange: &'a str,
    #[serde(borrow)]
    rate: &'a RawValue,
}

pub fn parse_lighter_funding(body: &str, market_id: u32) -> Result<HourlyRate, VenueError> {
    let rates: LighterRates<'_> = serde_json::from_str(body)?;
    let rate = rates
        .funding_rates
        .iter()
        .find(|r| r.market_id == market_id && r.exchange == "lighter")
        .ok_or_else(|| VenueError::MissingMarket(market_id.to_string()))?;
    let rate_8h = decimal_from_json_number(rate.rate.get())?
        .round_dp(12)
        .normalize();
    Ok(HourlyRate::from_8h(rate_8h)?)
}

#[derive(Deserialize)]
struct LighterDetails {
    order_book_details: Vec<LighterDetail>,
}

#[derive(Deserialize)]
struct LighterDetail {
    market_id: u32,
    #[serde(with = "rust_decimal::serde::str")]
    taker_fee: Decimal,
}

pub fn parse_lighter_taker_fee(body: &str, market_id: u32) -> Result<Decimal, VenueError> {
    let details: LighterDetails = serde_json::from_str(body)?;
    let detail = details
        .order_book_details
        .iter()
        .find(|d| d.market_id == market_id)
        .ok_or_else(|| VenueError::MissingMarket(market_id.to_string()))?;
    Ok(detail.taker_fee / Decimal::ONE_HUNDRED)
}

fn decimal_from_json_number(text: &str) -> Result<Decimal, VenueError> {
    let parsed = if text.contains(['e', 'E']) {
        Decimal::from_scientific(text)
    } else {
        Decimal::from_str(text)
    };
    parsed.map_err(|_| VenueError::BadNumber(text.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn json_numbers_parse_exactly() {
        assert_eq!(
            decimal_from_json_number("0.0001").unwrap(),
            Decimal::new(1, 4)
        );
        assert_eq!(
            decimal_from_json_number("9.6e-05").unwrap(),
            Decimal::new(96, 6)
        );
        assert!(matches!(
            decimal_from_json_number("NaN"),
            Err(VenueError::BadNumber(_))
        ));
    }
}
