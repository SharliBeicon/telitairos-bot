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

#[derive(PartialEq, Clone)]
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
