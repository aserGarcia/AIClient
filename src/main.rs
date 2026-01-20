use convo::screen::{Screen, conversation, loading};
use convo_core::directory;
use iced::{Size, Subscription, Task, time, window};
use std::time::Duration;
use tracing_subscriber::fmt;

const DEFUALT_REPO: &'static str = "Qwen/Qwen3-4B-GGUF";
const DEFAULT_MODEL: &'static str = "Qwen3-4B-Q5_K_M.gguf";

use std::path::PathBuf;

use iced::widget::{button, column, container, progress_bar, text};
use iced::{Element, Length};
use iced::task::{sipper, Sipper};
use futures::StreamExt;

fn main() -> iced::Result {

    fmt::init();
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
        .font(include_bytes!("../fonts/AveriaSerifLibre-Regular.ttf").as_slice())
        .font(include_bytes!("../fonts/OpenSans-VariableFont_wdth,wght.ttf").as_slice())
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
    Error
}

impl Convo {
    fn new() -> (Self, Task<Message>) {
        if let (loading, task) = loading::Loading::new() {
            (
                Self {
                    screen: Screen::Loading(loading),
                },
                task.map(Message::Loading),
            )
        } else {
            panic!("Could not load conversation.")
        }
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
                            return task.map(Message::Conversation)
                        } else {
                            self.screen = Screen::Error;
                            return Task::done(Message::Error)
                        }
                        
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
            Message::Error => {
                Task::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        match &self.screen {
            Screen::Loading(loading) => loading.view().map(Message::Loading),
            Screen::Conversation(conversation) => conversation.view().map(Message::Conversation),
            Screen::Error => text("Error").size(64).into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_secs(2))
            .map(|_| Message::Conversation(conversation::Message::AutoSave))
    }
}
