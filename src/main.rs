use convo::screen::{Screen, conversation, loading};
use iced::widget::text;
use iced::{Size, Subscription, Task, time, window};
use std::time::Duration;
use tracing::Level;
use tracing_subscriber::fmt;

fn main() -> iced::Result {
    let sub = fmt().with_max_level(Level::DEBUG).finish();
    tracing::subscriber::set_global_default(sub).expect("Failed to sub");
    iced::application(Convo::new, Convo::update, Convo::view)
        .title(Convo::title)
        .window(window::Settings {
            size: Size {
                width: 1500.0,
                height: 1000.0,
            },
            ..Default::default()
        })
        .font(include_bytes!("../fonts/chat-icons.ttf").as_slice())
        .font(include_bytes!("../fonts/Mooli-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/NotoSans-Regular.ttf").as_slice())
        .subscription(Convo::subscription)
        .run()
}

struct Convo {
    screen: Screen,
}

#[derive(Clone)]
enum Message {
    Loading(loading::Message),
    Conversation(conversation::Message),
    Error,
}

impl Convo {
    fn new() -> (Self, Task<Message>) {
        let (loading, task) = loading::Loading::new();
        (
            Self {
                screen: Screen::Loading(loading),
            },
            task.map(Message::Loading),
        )
    }

    fn title(&self) -> String {
        "Convo".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loading(message) => {
                let loading = if let Screen::Loading(loading) = &mut self.screen {
                    Some(loading)
                } else {
                    None
                };
                let Some(loading) = loading else {
                    return Task::none();
                };

                let action = loading.update(message);
                match action {
                    loading::Action::None => return Task::none(),
                    loading::Action::Run(task) => return task.map(Message::Loading),
                    loading::Action::Continue => {
                        if let Ok((conversation, task)) = conversation::Conversation::new() {
                            self.screen = Screen::Conversation(conversation);
                            return task.map(Message::Conversation);
                        } else {
                            self.screen =
                                Screen::Error("Error creating conversation struct".to_string());
                            return Task::done(Message::Error);
                        }
                    }
                    loading::Action::Error(e) => {
                        self.screen = Screen::Error(e);
                        return Task::done(Message::Error);
                    }
                }
            }
            Message::Conversation(message) => {
                let conversation = if let Screen::Conversation(conversation) = &mut self.screen {
                    Some(conversation)
                } else {
                    None
                };

                let Some(conversation) = conversation else {
                    return Task::none();
                };
                let action = conversation.update(message);

                match action {
                    conversation::Action::None => return Task::none(),
                    conversation::Action::Run(task) => return task.map(Message::Conversation),
                }
            }
            Message::Error => Task::none(),
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        match &self.screen {
            Screen::Loading(loading) => loading.view().map(Message::Loading),
            Screen::Conversation(conversation) => conversation.view().map(Message::Conversation),
            Screen::Error(e) => text(format!("Error: {}", e)).size(64).into(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_secs(2))
            .map(|_| Message::Conversation(conversation::Message::AutoSave))
    }
}
