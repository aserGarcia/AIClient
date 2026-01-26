use iced::widget::markdown;
use uuid::Uuid;

pub struct Chat {
    pub id: Uuid,
    pub title: String,
    pub minor_text: String,
    pub messages: Vec<ChatMessage>,
}

pub struct ChatMessage {
    pub id: usize,
    pub chat_id: Uuid,
    pub content: String,
    pub markdown: markdown::Content,
    pub is_reply: bool,
}

pub struct Reply {
    pub content: String,
    pub markdown: markdown::Content,
}
