use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    Space, button, column, container, operation, row, scrollable, text, text_editor, tooltip,
};
use iced::{Border, Color, Element, Font, Length, Task, Theme, color};
use iced_dialog::dialog;

use crate::styles::styles;

const CHAT_FONT: Font = Font::with_name("chat-icons");

pub struct Conversation {
    input: text_editor::Content,
    chats: Vec<Chat>,
    dialog_delete_chat_open: bool,
    dialog_delete_chat: Option<usize>,
    current_chat_id: Option<usize>,
}

#[derive(Clone)]
pub enum Message {
    Initialize,
    FocusInput,
    NewChat,
    OpenChat(usize),
    DeleteChat(Option<usize>),
    DialogDeleteChat(usize),
    DialogCancelDeleteChat,
    InputChange(text_editor::Action),
    SubmitMessage,
}

pub enum Action {
    None,
    Run(Task<Message>),
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

impl Conversation {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                input: text_editor::Content::new(),
                chats: Vec::new(),
                dialog_delete_chat_open: false,
                dialog_delete_chat: None,
                current_chat_id: None,
            },
            Task::done(Message::Initialize),
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Initialize => {
                return Action::Run(Task::batch([Task::done(Message::FocusInput)]));
            }
            Message::FocusInput => {
                let focus_input = operation::focus::<Message>("input");
                let scroll_to_recent = operation::snap_to_end::<Message>("conversation");
                return Action::Run(Task::batch([focus_input, scroll_to_recent]));
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
                return Action::Run(Task::done(Message::FocusInput));
            }
            Message::OpenChat(id) => {
                self.current_chat_id = Some(id);
                return Action::Run(Task::done(Message::FocusInput));
            }
            Message::DeleteChat(id) => {
                if let Some(id) = id {
                    self.chats.retain(|chat| chat.id != id);

                    if self.current_chat_id == Some(id) {
                        self.current_chat_id = if self.chats.len() > 0 {
                            Some(self.chats.len())
                        } else {
                            None
                        };
                    }
                    return Action::Run(Task::batch([
                        Task::done(Message::FocusInput),
                        Task::done(Message::DialogCancelDeleteChat),
                    ]));
                }
                return Action::None;
            }
            Message::DialogDeleteChat(id) => {
                self.dialog_delete_chat_open = true;
                self.dialog_delete_chat = Some(id);
                return Action::Run(Task::done(Message::FocusInput));
            }
            Message::DialogCancelDeleteChat => {
                self.dialog_delete_chat_open = false;
                self.dialog_delete_chat = None;
                return Action::Run(Task::done(Message::FocusInput));
            }
            Message::InputChange(action) => {
                self.input.perform(action);
                return Action::None;
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
                return Action::None;
            }
        };
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        //
        // Sidebar Widgets
        //

        // Recent Messages
        let chat_list: Vec<Element<Message>> = self
            .chats
            .iter()
            .rev()
            .map(|chat| {
                let delete_chat_button = button(text("\u{F146}").font(CHAT_FONT).size(12))
                    .on_press(Message::DialogDeleteChat(chat.id))
                    .style(styles::delete_chat_button);
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
                    delete_chat_button
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
                    text("Select a conversation or begin a new one.")
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

        let base = container(content).width(Length::Fill).height(Length::Fill);

        dialog(
            self.dialog_delete_chat_open,
            base,
            text("Would you like to delete the conversation?"),
        )
        .title("Delete Chat")
        .push_button(
            iced_dialog::button("Delete", Message::DeleteChat(self.dialog_delete_chat))
                .style(styles::chat_selected),
        )
        .push_button(
            iced_dialog::button("Cancel", Message::DialogCancelDeleteChat)
                .style(styles::chat_selected),
        )
        .on_press(Message::DialogCancelDeleteChat)
        .into()
    }
}
