use convo::screen::{Screen, conversation};

use iced::{Size, Task, window};

fn main() -> iced::Result {
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
        .run()
}

// Drives the dynamic state of the GUI
struct Convo {
    screen: Screen,
}

#[derive(Clone)]
enum Message {
    Conversation(conversation::Message),
}

impl Convo {
    fn new() -> (Self, Task<Message>) {
        let (conversation, task) = conversation::Conversation::new();
        (
            Self {
                screen: Screen::Conversation(conversation),
            },
            task.map(Message::Conversation),
        )
    }

    fn title(&self) -> String {
        "Convo".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
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
        };
    }

    fn view(&self) -> iced::Element<'_, Message> {
        match &self.screen {
            Screen::Conversation(conversation) => conversation.view().map(Message::Conversation),
        }
    }
}
