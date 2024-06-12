use std::{
    collections::{HashMap, VecDeque},
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
