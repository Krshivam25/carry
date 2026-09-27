use std::error::Error;

use carry_venues::rest::{http_client, hyperliquid_quote, lighter_quote};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = http_client()?;
    let (hl, lighter) = tokio::join!(hyperliquid_quote(&client, "BTC"), lighter_quote(&client, 1));
    for (name, quote) in [("hyperliquid", hl?), ("lighter", lighter?)] {
        println!(
            "{name:<12} funding {}/h ({}% APR)  taker fee {}",
            quote.funding.value(),
            (quote.funding.apr()? * rust_decimal::Decimal::ONE_HUNDRED).round_dp(2),
            quote.taker_fee
        );
    }
    Ok(())
}
