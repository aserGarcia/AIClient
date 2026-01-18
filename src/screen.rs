pub mod loading;
pub mod conversation;

pub use conversation::Conversation;
pub use loading::Loading;
pub enum Screen {
    Conversation(Conversation),
    Loading(Loading)
}
