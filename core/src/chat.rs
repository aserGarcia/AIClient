use uuid::Uuid;

pub struct Chat {
    pub id: Uuid,
    pub title: String,
    pub messages: Vec<ChatMessage>,
}

pub struct ChatMessage {
    pub id: usize,
    pub chat_id: Uuid,
    pub content: String,
    pub is_reply: bool,
}
