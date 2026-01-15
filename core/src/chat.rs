pub struct Chat {
    pub id: usize,
    pub title: String,
    pub messages: Vec<ChatMessage>,
}

pub struct ChatMessage {
    pub id: usize,
    pub chat_id: usize,
    pub content: String,
    pub is_reply: bool,
}
