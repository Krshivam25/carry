mod book_owner;
mod markets;
mod pipeline;
mod run;
mod scan;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use crate::scan::ScanArgs;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run,
    Scan(ScanArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let default_level = match cli.command {
        Command::Run => "info",
        Command::Scan(_) => "warn",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level)),
        )
        .init();

    match cli.command {
        Command::Run => run::run().await,
        Command::Scan(args) => scan::scan(args).await,
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;
    use rust_decimal::Decimal;

    use super::*;
    use crate::markets::Coin;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn scan_parses_arguments() {
        let cli = Cli::try_parse_from(["carryd", "scan", "--coin", "eth", "--notional", "1000.5"])
            .unwrap();
        let Command::Scan(args) = cli.command else {
            panic!("expected scan");
        };
        assert_eq!(args.coin, Coin::Eth);
        assert_eq!(args.notional, Decimal::new(10_005, 1));
        assert_eq!(args.hours, Decimal::new(168, 0));
    }

    #[test]
    fn scan_defaults() {
        let cli = Cli::try_parse_from(["carryd", "scan"]).unwrap();
        let Command::Scan(args) = cli.command else {
            panic!("expected scan");
        };
        assert_eq!(args.coin, Coin::Btc);
        assert_eq!(args.notional, Decimal::new(50_000, 0));
    }

    #[test]
    fn bad_notional_is_rejected() {
        assert!(Cli::try_parse_from(["carryd", "scan", "--notional", "abc"]).is_err());
        assert!(Cli::try_parse_from(["carryd", "scan", "--coin", "doge"]).is_err());
    }
}
