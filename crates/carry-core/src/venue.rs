use std::fmt;
use std::str::FromStr;

use crate::CarryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Venue {
    Hyperliquid,
    Lighter,
}

impl Venue {
    pub const ALL: [Self; 2] = [Self::Hyperliquid, Self::Lighter];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hyperliquid => "hyperliquid",
            Self::Lighter => "lighter",
        }
    }
}

impl fmt::Display for Venue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Venue {
    type Err = CarryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "hyperliquid" | "hl" => Ok(Self::Hyperliquid),
            "lighter" => Ok(Self::Lighter),
            _ => Err(CarryError::UnknownVenue(s.to_owned())),
        }
    }
}
