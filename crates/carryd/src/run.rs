use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use carry_core::{OrderBook, Side};
use carry_venues::FeedConfig;
use rust_decimal::Decimal;
use tokio::task::JoinSet;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::markets::Coin;
use crate::pipeline::{spawn_venue, stop};

const REPORT_EVERY: Duration = Duration::from_secs(10);

pub async fn run() -> Result<()> {
    let shutdown = CancellationToken::new();
    let mut tasks = JoinSet::new();
    let market = Coin::Btc.market();
    let hl = spawn_venue(
        &mut tasks,
        FeedConfig::hyperliquid(market.hl_coin, market.tick_size),
        &shutdown,
    )?;
    let lighter = spawn_venue(
        &mut tasks,
        FeedConfig::lighter(market.lighter_id, market.tick_size),
        &shutdown,
    )?;
    info!("carryd started");

    let notional = Decimal::new(50_000, 0);
    let mut report = interval(REPORT_EVERY);
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = report.tick() => {
                let hl_book = Arc::clone(&hl.borrow());
                let lighter_book = Arc::clone(&lighter.borrow());
                log_executable("hyperliquid", &hl_book, notional);
                log_executable("lighter", &lighter_book, notional);
            }
            _ = &mut ctrl_c => break,
        }
    }
    info!("shutting down");
    stop(tasks, shutdown).await;
    Ok(())
}

fn log_executable(venue: &str, book: &OrderBook, notional: Decimal) {
    if !book.is_synced() {
        warn!(venue, "book stale");
        return;
    }
    match (
        book.walk(Side::Ask, notional),
        book.walk(Side::Bid, notional),
    ) {
        (Ok(buy), Ok(sell)) => info!(
            venue,
            %notional,
            buy_avg = %buy.avg_price.round_dp(1),
            buy_bps = %buy.slippage_bps.round_dp(2),
            sell_avg = %sell.avg_price.round_dp(1),
            sell_bps = %sell.slippage_bps.round_dp(2),
            "executable"
        ),
        (Err(err), _) | (_, Err(err)) => warn!(venue, %err, "walk failed"),
    }
}
