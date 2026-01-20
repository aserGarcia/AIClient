pub mod conversation;
pub mod loading;

pub use conversation::Conversation;
pub use loading::Loading;

pub enum Screen {
    Conversation(Conversation),
    Loading(Loading),
    Error
}
