pub mod admin;
pub mod ai;

use crate::{types, TelitairoBot};
use std::collections::VecDeque;
use teloxide::prelude::*;

pub async fn handle_messages(
    buffer_store: types::BufferStore,
    telitairo_bot: TelitairoBot,
    msg: Message,
) -> ResponseResult<()> {
    let mut buffer_store_lock = buffer_store.write().await;
    match buffer_store_lock.get_mut(&msg.chat.id) {
        Some(buffer) => {
            if buffer.len() == telitairo_bot.buffer_size {
                buffer.pop_front();
            }
            buffer.push_back(msg.clone());
        }
        None => {
            let mut buffer = VecDeque::new();
            buffer.push_back(msg.clone());
            buffer_store_lock.insert(msg.chat.id, buffer);
        }
    }

    Ok(())
}
