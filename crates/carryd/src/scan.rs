use std::sync::Arc;
use std::time::Duration;

use crate::markets::Coin;
use crate::pipeline::{spawn_venue, stop};
use anyhow::{Context, Result};
use carry_core::{ExecutableCarry, Leg, OrderBook, executable_carry};
use carry_venues::FeedConfig;
use carry_venues::rest::{VenueQuote, http_client, hyperliquid_quote, lighter_quote};
use clap::Args;
use rust_decimal::Decimal;
use tokio::sync::watch;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

const SYNC_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Args)]
pub struct ScanArgs {
    /// Market to price.
    #[arg(long, value_enum, default_value_t = Coin::Btc)]
    pub coin: Coin,
    /// Quote notional per leg, in USD.
    #[arg(long, default_value = "50000")]
    pub notional: Decimal,
    /// Holding period in hours for the net-carry column.
    #[arg(long, default_value = "168")]
    pub hours: Decimal,
}

pub async fn scan(args: ScanArgs) -> Result<()> {
    let market = args.coin.market();
    let client = http_client()?;
    let shutdown = CancellationToken::new();
    let mut tasks = JoinSet::new();
    let hl_view = spawn_venue(
        &mut tasks,
        FeedConfig::hyperliquid(market.hl_coin, market.tick_size),
        &shutdown,
    )?;

    let lighter_view = spawn_venue(
        &mut tasks,
        FeedConfig::lighter(market.lighter_id, market.tick_size),
        &shutdown,
    )?;

    let (hl_quote, lighter_quote, hl_book, lighter_book) = tokio::join!(
        hyperliquid_quote(&client, market.hl_coin),
        lighter_quote(&client, market.lighter_id),
        wait_synced(hl_view),
        wait_synced(lighter_view),
    );

    stop(tasks, shutdown).await;

    let hl_quote = hl_quote.context("hyperliquid funding/fees")?;
    let lighter_quote = lighter_quote.context("lighter funding/fees")?;
    let hl_book = hl_book.context("hyperliquid book")?;
    let lighter_book = lighter_book.context("lighter book")?;

    let hl = leg(&hl_book, hl_quote);
    let lighter = leg(&lighter_book, lighter_quote);
    let rows = [
        (
            "long lighter / short hl",
            executable_carry(&lighter, &hl, args.notional)?,
        ),
        (
            "long hl / short lighter",
            executable_carry(&hl, &lighter, args.notional)?,
        ),
    ];

    println!(
        "{:?} notional ${} per leg hold {} h",
        args.coin, args.notional, args.hours
    );

    println!(
        "{:<24} {:>13} {:>9} {:>11} {:>11} {:>11} {:>13}",
        "direction", "spread/h", "APR %", "entry $", "round $", "breakeven h", "net carry $"
    );
    for (name, carry) in rows {
        print_row(name, &carry, args.hours)?;
    }
    Ok(())
}

fn leg(book: &OrderBook, quote: VenueQuote) -> Leg<'_> {
    Leg {
        book,
        funding: quote.funding,
        taker_fee: quote.taker_fee,
    }
}

async fn wait_synced(mut view: watch::Receiver<Arc<OrderBook>>) -> Result<Arc<OrderBook>> {
    let book = timeout(SYNC_TIMEOUT, view.wait_for(|book| book.is_synced()))
        .await
        .context("timed out waiting for a synced book")?
        .context("book owner stopped")?;
    Ok(Arc::clone(&book))
}

fn print_row(name: &str, carry: &ExecutableCarry, hours: Decimal) -> Result<()> {
    let apr_pct = carry.hourly_spread.apr()? * Decimal::ONE_HUNDRED;
    let breakeven = carry
        .breakeven_hours
        .map_or_else(|| "—".to_owned(), |h| h.round_dp(1).to_string());
    println!(
        "{:<24} {:>13} {:>9} {:>11} {:>11} {:>11} {:>13}",
        name,
        carry.hourly_spread.value().round_dp(8),
        apr_pct.round_dp(2),
        carry.entry_cost.round_dp(2),
        carry.round_trip_cost.round_dp(2),
        breakeven,
        carry.net_carry(hours)?.round_dp(2),
    );
    Ok(())
}
