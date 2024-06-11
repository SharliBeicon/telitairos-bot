pub mod bot;
pub mod gpt;

pub mod types {
    use std::{collections::VecDeque, sync::Arc};
    use teloxide::types::Message;
    use tokio::sync::RwLock;

    pub const PERSONALITY: &str= "Eres un asistente andaluz con jerga informal y algo irónica. Ayudas a todo aquel que te necesite, no sin antes quejarte un poco, ya que eres algo vago.";
    pub const MEDIATE_QUERY: &str= "A partir de los siguientes mensajes, analiza una posible discusión y da la razón a alguno de los implicados, con una pequeña argumentación.";
    pub const STORE_CAPACITY: usize = 200;

    pub type Messages = Arc<RwLock<VecDeque<Message>>>;
}
