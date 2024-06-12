use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use teloxide::types::{ChatId, Message};
use tokio::sync::RwLock;

pub type BufferStore = Arc<RwLock<HashMap<ChatId, VecDeque<Message>>>>;
