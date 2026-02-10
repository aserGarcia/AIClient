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

impl Default for ChatMessage {
    fn default() -> ChatMessage {
        ChatMessage {
            id: 0,
            chat_id: Uuid::new_v4(),
            content: String::new(),
            markdown: markdown::Content::new(),
            is_reply: false,
        }
    }
}
