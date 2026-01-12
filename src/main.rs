use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Space, button, column, container, operation, row, scrollable, text, text_editor, tooltip,
};
use iced::{Color, Element, Font, Length, Size, Task, window};

const CHAT_FONT: Font = Font::with_name("chat-icons");

fn main() -> iced::Result {
    iced::application(SecureClient::new, SecureClient::update, SecureClient::view)
        .title(SecureClient::title)
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
struct SecureClient {
    input: text_editor::Content,
    chats: Vec<Chat>,
    current_chat_id: Option<usize>,
}

#[derive(Clone)]
enum Message {
    Initialize,
    NewChat,
    OpenChat(usize),
    DeleteChat(usize),
    InputChange(text_editor::Action),
    SubmitMessage,
}

struct Chat {
    id: usize,
    title: String,
    messages: Vec<ChatMessage>,
}

struct ChatMessage {
    content: String,
    is_reply: bool,
}

impl Default for SecureClient {
    fn default() -> Self {
        Self {
            input: text_editor::Content::new(),
            chats: Vec::new(),
            current_chat_id: None,
        }
    }
}

impl SecureClient {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                input: text_editor::Content::new(),
                chats: Vec::new(),
                current_chat_id: None,
            },
            Task::done(Message::Initialize),
        )
    }

    fn title(&self) -> String {
        "SecureClient AI".to_string()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Initialize => {
                let focus_input = operation::focus::<Message>("input");
                let scroll_to_recent = operation::snap_to_end::<Message>("conversation");
                return Task::batch([focus_input, scroll_to_recent]);
            }
            Message::NewChat => {
                let idx = self.chats.len() + 1;
                let chat = Chat {
                    id: idx,
                    title: format!("Chat {}", idx),
                    messages: Vec::new(),
                };
                self.current_chat_id = Some(idx);
                self.chats.push(chat);
                return Task::none();
            }
            Message::OpenChat(id) => {
                self.current_chat_id = Some(id);
                return Task::none();
            }
            Message::DeleteChat(id) => {
                self.chats.retain(|chat| chat.id != id);

                if self.current_chat_id == Some(id) {
                    self.current_chat_id = if self.chats.len() > 0 {
                        Some(self.chats.len())
                    } else {
                        None
                    };
                }
                return Task::none();
            }
            Message::InputChange(action) => {
                self.input.perform(action);
                return Task::none();
            }
            Message::SubmitMessage => {
                if self.input.text().len() > 0 {
                    let message = ChatMessage {
                        content: self.input.text(),
                        is_reply: false,
                    };

                    let default_reply = ChatMessage {
                        content: "This is a default AI reply".to_string(),
                        is_reply: true,
                    };

                    if let Some(id) = self.current_chat_id {
                        if let Some(idx) = self.chats.iter().position(|x| x.id == id) {
                            self.chats[idx].messages.push(message);
                            self.chats[idx].messages.push(default_reply);
                        }
                    } else {
                        let idx = self.chats.len() + 1;
                        self.current_chat_id = Some(idx);
                        self.chats.push(Chat {
                            id: idx,
                            title: format!("Chat {}", idx),
                            messages: vec![message],
                        });
                    }

                    self.input = text_editor::Content::new();
                }
                return Task::none();
            }
        };
    }

    fn view(&self) -> iced::Element<'_, Message> {
        //
        // Sidebar Widgets
        //

        //
        // Recent Messages
        //

        let chat_list: Vec<Element<Message>> = self
            .chats
            .iter()
            .map(|chat| {
                let mut chat_button =
                    button(text(chat.title.clone()).size(13)).on_press(Message::OpenChat(chat.id));
                if Some(chat.id) == self.current_chat_id {
                    chat_button = chat_button.style(styles::chat_selected);
                } else {
                    chat_button = chat_button.style(styles::open_chat_button);
                }
                row![
                    chat_button,
                    Space::new().width(Length::Fill),
                    button(text("\u{F146}").font(CHAT_FONT).size(12))
                        .on_press(Message::DeleteChat(chat.id))
                        .style(styles::delete_chat_button)
                ]
                .align_y(Vertical::Center)
                .into()
            })
            .collect();

        let chat_messages = column(chat_list)
            .spacing(10)
            .padding(10)
            .height(Length::Fill);

        let settings = row![
            container(text("🟢").size(24)),
            container(column![text("UserId").size(16)])
        ]
        .align_y(Vertical::Center)
        .spacing(10);
        let recent_messages = container(column![chat_messages, settings]);

        //
        // New chat button
        //
        let new_chat = tooltip(
            button(text('\u{F0FE}').font(CHAT_FONT).size(12))
                .on_press(Message::NewChat)
                .style(styles::new_chat_button),
            text("New chat").size(12),
            tooltip::Position::Right,
        );

        let sidebar = container(
            column![
                row![
                    text("Recent conversations").size(12),
                    Space::new().width(Length::Fill),
                    new_chat
                ]
                .align_y(Vertical::Center),
                recent_messages
            ]
            .padding(20),
        )
        .width(200)
        .height(Length::Fill)
        .style(styles::sidebar);

        //
        // Conversation
        //
        let conversation = if let Some(chat_id) = self.current_chat_id {
            if let Some(chat) = self.chats.iter().find(|c| c.id == chat_id) {
                let messages: Vec<iced::Element<Message>> = chat
                    .messages
                    .iter()
                    .map(|msg| {
                        let text = container(text(msg.content.clone()).size(14)).padding(15);
                        if !msg.is_reply {
                            row![
                                Space::new().width(Length::Fill),
                                text.style(styles::message)
                            ]
                            .align_y(Vertical::Center)
                            .into()
                        } else {
                            row![text, Space::new().width(Length::Fill)]
                                .align_y(Vertical::Center)
                                .into()
                        }
                    })
                    .collect();

                container(scrollable(column(messages).spacing(10).padding(20)).id("conversation"))
            } else {
                container(
                    text("Select a chat from the sidebar.")
                        .size(24)
                        .color(Color::WHITE),
                )
                .center_y(Length::Fill)
                .center_x(Length::Fill)
            }
        } else {
            container(
                text("Type and hit enter to begin a conversation.")
                    .size(24)
                    .color(Color::WHITE),
            )
            .center_y(Length::Fill)
            .center_x(Length::Fill)
        };

        let conversation = conversation.height(Length::FillPortion(6)).max_width(800);

        //
        // Input Field
        //
        //
        let text_editor_field = container(
            text_editor(&self.input)
                .id("input")
                .placeholder("Type something...")
                .on_action(Message::InputChange)
                .key_binding(|key_press| {
                    let modifiers = key_press.modifiers;

                    match text_editor::Binding::from_key_press(key_press) {
                        Some(text_editor::Binding::Enter) if !modifiers.shift() => {
                            Some(text_editor::Binding::Custom(Message::SubmitMessage))
                        }
                        binding => binding,
                    }
                })
                .style(styles::text_editor_field),
        )
        .padding(20)
        .align_y(Vertical::Center)
        .width(Length::FillPortion(2))
        .height(400);

        let input_field = container(row![
            Space::new().width(Length::FillPortion(1)),
            text_editor_field,
            Space::new().width(Length::FillPortion(1))
        ])
        .height(Length::FillPortion(1));

        //
        // Messaging Area
        //
        let messaging_area =
            container(column![conversation, input_field].align_x(Horizontal::Center))
                .width(Length::Fill)
                .style(styles::messaging_area);

        //
        // Main area
        //
        let main_area = container(row![sidebar, messaging_area]).height(Length::Fill);

        let content = column![main_area];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

mod styles {
    use iced::widget::{button, container, text_editor};
    use iced::{Border, Color, Theme, color};

    pub fn sidebar(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(color!(0xffffe3).into()),
            background: Some(color!(0x080b05).into()),
            border: Border {
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn messaging_area(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(color!(0x050301).into()),
            border: Border {
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn message(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(color!(0xffffe3).into()),
            background: Some(color!(0x080b05).into()),
            border: Border {
                radius: 20.0.into(),
                color: color!(0x93b1a6, 0.5),
                width: 1.5,
                ..Default::default()
            },

            ..Default::default()
        }
    }

    pub fn new_chat_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered => button::Style {
                text_color: color!(0x93b1a6).into(),
                ..Default::default()
            },
            _ => button::Style {
                text_color: Color::WHITE,
                ..Default::default()
            },
        }
    }

    pub fn delete_chat_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered => button::Style {
                text_color: color!(0x93b1a6).into(),
                ..Default::default()
            },
            _ => button::Style {
                text_color: Color::WHITE,
                ..Default::default()
            },
        }
    }

    pub fn open_chat_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered => button::Style {
                text_color: Color::BLACK,
                border: Border {
                    radius: 5.0.into(),
                    ..Default::default()
                },
                background: Some(color!(0x93b1a6).into()),
                ..Default::default()
            },
            _ => button::Style {
                text_color: Color::WHITE,
                border: Border {
                    radius: 5.0.into(),
                    ..Default::default()
                },
                background: Some(color!(0x080b05).into()),
                ..Default::default()
            },
        }
    }

    pub fn chat_selected(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered | button::Status::Active => button::Style {
                text_color: Color::BLACK,
                border: Border {
                    radius: 5.0.into(),
                    ..Default::default()
                },
                background: Some(color!(0x93b1a6).into()),
                ..Default::default()
            },
            _ => button::Style {
                text_color: Color::WHITE,
                border: Border {
                    radius: 5.0.into(),
                    ..Default::default()
                },
                background: Some(color!(0x080b05).into()),
                ..Default::default()
            },
        }
    }

    pub fn text_editor_field(_theme: &Theme, status: text_editor::Status) -> text_editor::Style {
        text_editor::Style {
            background: color!(0x93B1A6).into(),
            border: Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            placeholder: color!(0x3c4a45).into(),
            value: Color::BLACK.into(),
            selection: Color::WHITE.into(),
        }
    }
}
