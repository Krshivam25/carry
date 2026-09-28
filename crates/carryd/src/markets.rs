use clap::ValueEnum;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Coin {
    Btc,
    Eth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Market {
    pub hl_coin: &'static str,
    pub lighter_id: u32,
    pub tick_size: Decimal,
}

impl Coin {
    pub fn market(self) -> Market {
        match self {
            Self::Btc => Market {
                hl_coin: "BTC",
                lighter_id: 1,
                tick_size: Decimal::new(1, 1),
            },

            Self::Eth => Market {
                hl_coin: "ETH",
                lighter_id: 0,
                tick_size: Decimal::new(1, 2),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markets_map_to_venue_ids() {
        assert_eq!(Coin::Btc.market().lighter_id, 1);
        assert_eq!(Coin::Eth.market().hl_coin, "ETH");
        assert_eq!(Coin::Eth.market().tick_size, Decimal::new(1, 2));
    }
}
