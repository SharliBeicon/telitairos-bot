use std::{
    collections::{HashMap, VecDeque},
    str::FromStr,
    sync::Arc,
};
use teloxide::types::{ChatId, Message};
use tokio::sync::RwLock;

pub const DEFAULT_PERSONALITY: &str =
    "You are a virtual assistant with a touch of acid humour and you love potatoes";
pub const DEFAULT_MEDIATION_QUERY: &str =
    "Take the messages, search for possible discussions and choose one side";
pub const DEFAULT_BUFFER_SIZE: usize = 200;

pub type BufferStore = Arc<RwLock<HashMap<ChatId, VecDeque<Message>>>>;

pub type TimeAmount = u8;

#[derive(Clone, Debug, PartialEq)]
pub enum UnitOfTime {
    Seconds,
    Minutes,
    Hours,
    Permanent,
}

impl FromStr for UnitOfTime {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "h" => Ok(UnitOfTime::Hours),
            "m" => Ok(UnitOfTime::Minutes),
            "s" => Ok(UnitOfTime::Seconds),
            "p" => Ok(UnitOfTime::Permanent),
            _ => Err("Allowed units: h, m, s, p"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_ok() {
        let uot: UnitOfTime = UnitOfTime::from_str("h").expect("Failed to convert");
        assert_eq!(uot, UnitOfTime::Hours);
    }

    #[test]
    fn conversion_nok() {
        let result = UnitOfTime::from_str("x");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Allowed units: h, m, s, p");
    }
}
