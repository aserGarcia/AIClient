use convo_core::chat::CompletionMessage;
use iced::alignment::{Horizontal, Vertical};
use iced::task::{Sipper, sipper};
use iced::widget::{
    Space, button, column, container, markdown, operation, right, row, scrollable, text,
    text_editor,
};
use iced::{Element, Font, Length, Task, Theme};
use iced_dialog::dialog;

use thiserror::Error;
use uuid::Uuid;

use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};

use crate::styles::{styles, viewers};
use convo_core::{
    assistant::{Chatting, LlamaCpp},
    chat::{Chat, ChatMessage, Reply},
    db,
};

const CHAT_FONT: Font = Font::with_name("chat-icons");
const MOOLI: Font = Font::with_name("Mooli");
const NOTO_SANS: Font = Font::with_name("Noto Sans");

pub struct Conversation {
    server: Option<Arc<Mutex<LlamaCpp>>>,
    server_ready: bool,
    input: text_editor::Content,
    replying_string: Reply,
    chats: Vec<Chat>,
    db: db::Database,
    dialog_delete_chat_open: bool,
    dialog_delete_chat: Option<Uuid>,
    current_chat_id: Option<Uuid>,
}

#[derive(Clone)]
pub enum Message {
    Initialize(Status),
    FocusInput,
    Markdown(viewers::Interaction),
    NewChat,
    OpenChat(Uuid),
    DeleteChat(Option<Uuid>),
    DialogDeleteChat(Uuid),
    DialogCancelDeleteChat,
    InputChange(text_editor::Action),
    SubmitMessage,
    ReplyMode(Chatting),
    AutoSave,
}

#[derive(PartialEq, Clone)]
pub enum Status {
    Loading,
    Loaded,
    Error(String),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Error(String),
}

#[derive(Error, Debug)]
pub enum ConversationError {
    #[error("Failed to load conversations: {0}")]
    Loading(String),
}

impl Conversation {
    pub fn new() -> Result<(Self, Task<Message>), ConversationError> {
        // handle error more gracefully
        let db = db::Database::new().map_err(|e| ConversationError::Loading(e.to_string()))?;
        debug!("db loaded");

        let chats = db
            .load_chats()
            .map_err(|e| ConversationError::Loading(e.to_string()))?;
        debug!("chats loaded {}", chats.len());

        Ok((
            Self {
                server: None,
                server_ready: false,
                input: text_editor::Content::new(),
                replying_string: Reply {
                    content: String::new(),
                    markdown: markdown::Content::new(),
                },
                chats,
                db,
                dialog_delete_chat_open: false,
                dialog_delete_chat: None,
                current_chat_id: None,
            },
            Task::done(Message::Initialize(Status::Loading)),
        ))
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::Initialize(status) => match status {
                Status::Loading => {
                    debug!("Starting server");
                    let server = match LlamaCpp::boot() {
                        Ok(llama) => llama,
                        Err(e) => {
                            return Action::Error(e.to_string());
                        }
                    };
                    let server = Arc::new(Mutex::new(server));
                    self.server = Some(Arc::clone(&server));

                    debug!("Awaiting server");
                    let wait_task = Task::perform(
                        async move {
                            let mut s = server.lock().await;
                            match s.wait_until_ready().await {
                                Ok(_) => Message::Initialize(Status::Loaded),
                                Err(e) => {
                                    error!("Server failed to become ready: {}", e);
                                    Message::Initialize(Status::Error(e.to_string()))
                                }
                            }
                        },
                        |msg| msg,
                    );
                    return Action::Run(wait_task);
                }
                Status::Loaded => {
                    self.server_ready = true;
                    return Action::Run(Task::done(Message::FocusInput));
                }
                Status::Error(e) => {
                    return Action::Error(e);
                }
            },
            Message::Markdown(interaction) => {
                return Action::Run(interaction.perform());
            }
            Message::FocusInput => {
                let focus_input = operation::focus::<Message>("input");
                let scroll_to_recent = operation::snap_to_end::<Message>("conversation");
                return Action::Run(Task::batch([focus_input, scroll_to_recent]));
            }
            Message::NewChat => {
                let uuid = Uuid::new_v4();
                let chat = Chat {
                    id: uuid,
                    title: String::new(),
                    messages: Vec::new(),
                };
                self.current_chat_id = Some(uuid);

                if let Err(e) = self.db.save_chat(&chat) {
                    error!("Failed to save new chat: {}", e);
                    return Action::None;
                }
                self.chats.push(chat);
                return Action::Run(Task::done(Message::FocusInput));
            }
            Message::OpenChat(id) => {
                self.current_chat_id = Some(id);
                return Action::Run(Task::done(Message::FocusInput));
            }
            Message::DeleteChat(id) => {
                if let Some(id) = id {
                    if let Err(e) = self.db.delete_chat(&id) {
                        error!("Failed to delete chat {}", e.to_string());
                        return Action::None;
                    }
                    self.chats.retain(|chat| chat.id != id);

                    self.current_chat_id = None;
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
                if !self.input.text().trim().is_empty() {
                    if let Some(id) = self.current_chat_id {
                        if let Some(idx) = self.chats.iter().position(|x| x.id == id) {
                            let msg_id = self.chats[idx].messages.len() + 1;
                            let message = ChatMessage {
                                id: msg_id,
                                chat_id: id,
                                content: self.input.text(),
                                markdown: markdown::Content::parse(self.input.text().as_str()),
                                is_reply: false,
                            };

                            if self.chats[idx].title.is_empty() {
                                let title = format!("{:.15}...", self.input.text());
                                self.chats[idx].title.push_str(title.as_str());
                            }

                            self.chats[idx].messages.push(message);
                        }
                    } else {
                        let chat_id = Uuid::new_v4();
                        self.current_chat_id = Some(chat_id);
                        let message = ChatMessage {
                            id: 0,
                            chat_id: chat_id,
                            content: self.input.text(),
                            markdown: markdown::Content::parse(self.input.text().as_str()),
                            is_reply: false,
                        };

                        self.chats.push(Chat {
                            id: chat_id,
                            title: format!("{:.15}...", self.input.text()),
                            messages: vec![message],
                        });
                    }

                    self.input = text_editor::Content::new();
                    let chat_idx = self
                        .chats
                        .iter()
                        .position(|x| x.id == self.current_chat_id.unwrap())
                        .unwrap();

                    let len = self.chats[chat_idx].messages.len();
                    let start = std::cmp::min(len - 1, 3);
                    let messages: Vec<CompletionMessage> = self.chats[chat_idx].messages[start..]
                        .iter()
                        .map(|m| CompletionMessage {
                            content: m.content.clone(),
                            is_reply: m.is_reply,
                        })
                        .collect();

                    if let Some(server) = &self.server {
                        let server = Arc::clone(server);
                        return Action::Run(Task::stream(reply_stream(server, messages)));
                    } else {
                        error!("Server not initialized");
                        return Action::Error("Server not initialized".to_string());
                    }
                }
                return Action::None;
            }
            Message::ReplyMode(message) => {
                match message {
                    Chatting::Token(tok) => {
                        self.replying_string.content.push_str(tok.as_str());
                        self.replying_string.markdown.push_str(tok.as_str());
                        return Action::Run(Task::done(Message::FocusInput));
                    }
                    Chatting::Complete => {
                        if let Some(id) = self.current_chat_id {
                            if let Some(idx) = self.chats.iter().position(|x| x.id == id) {
                                // Find the last reply message and append the token
                                let msg_id = self.chats[idx].messages.len() + 1;
                                let message = ChatMessage {
                                    id: msg_id,
                                    chat_id: id,
                                    content: self.replying_string.content.clone(),
                                    markdown: markdown::Content::parse(
                                        self.replying_string.content.as_str(),
                                    ),
                                    is_reply: true,
                                };
                                self.chats[idx].messages.push(message);

                                self.replying_string.content.clear();
                                self.replying_string.markdown = markdown::Content::new();
                            }
                        }

                        self.db.needs_save = true;
                        return Action::Run(Task::done(Message::FocusInput));
                    }
                    Chatting::Error(e) => {
                        error!("Generation error: {e}");
                        return Action::None;
                    }
                }
            }
            Message::AutoSave => {
                if self.db.needs_save {
                    for chat in &self.chats {
                        debug!("processing chat {}", chat.id);
                        if let Err(e) = self.db.save_chat(chat) {
                            error!("Failed to auto-save chat: {}", e);
                        }
                    }
                    self.db.needs_save = false;
                    println!("Autosave complete")
                }
                return Action::None;
            }
        };
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        if !self.server_ready {
            let page = container(column![
                text("Convo")
                    .color(styles::text_color())
                    .font(MOOLI)
                    .size(64),
                Space::new().height(10.0)
            ])
            .center(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(styles::background_color().into()),
                ..Default::default()
            });
            return page.into();
        }
        //
        // Sidebar Widgets
        //

        // Recent Messages
        let chat_list: Vec<Element<Message>> = self
            .chats
            .iter()
            .rev()
            .map(|chat| {
                let delete_chat_button = button(
                    text("\u{F146}")
                        .color(styles::text_color())
                        .font(CHAT_FONT)
                        .size(16),
                )
                .on_press(Message::DialogDeleteChat(chat.id))
                .style(styles::delete_chat_button);

                let mut chat_button =
                    button(container(text(chat.title.clone()).font(NOTO_SANS).size(16)))
                        .on_press(Message::OpenChat(chat.id));

                chat_button = if Some(chat.id) == self.current_chat_id {
                    chat_button.style(styles::chat_selected)
                } else {
                    chat_button.style(styles::open_chat_button)
                };

                let mut chat_container = container(
                    row![
                        chat_button,
                        Space::new().width(Length::Fill),
                        delete_chat_button
                    ]
                    .align_y(Vertical::Center),
                )
                .padding(5);

                chat_container = if Some(chat.id) == self.current_chat_id {
                    chat_container.style(styles::chat_container_selected)
                } else {
                    chat_container.style(styles::chat_container_default)
                };
                chat_container.into()
            })
            .collect();

        let chat_messages = column(chat_list).height(Length::Fill);

        let recent_messages = container(column![chat_messages]);

        //
        // New chat button
        //
        let new_chat = button(
            text('\u{F0FE}')
                .font(CHAT_FONT)
                .size(16)
                .color(styles::primary_color()),
        )
        .on_press(Message::NewChat)
        .style(styles::new_chat_button);

        let sidebar = container(column![
            container(
                row![
                    text("Convo").font(MOOLI).size(24),
                    Space::new().width(Length::Fill),
                    new_chat
                ]
                .align_y(Vertical::Center)
            )
            .style(styles::convo_header)
            .padding(10),
            recent_messages
        ])
        .padding(1)
        .width(195)
        .height(Length::Fill)
        .style(styles::sidebar);

        //
        // Conversation
        //
        let conversation = if let Some(chat_id) = self.current_chat_id {
            if let Some(chat) = self.chats.iter().find(|c| c.id == chat_id) {
                let mut messages: Vec<iced::Element<Message>> = chat
                    .messages
                    .iter()
                    .map(|msg| {
                        let text = container(
                            markdown::view_with(
                                msg.markdown.items(),
                                Theme::GruvboxLight,
                                &viewers::MarkdownViewer {},
                            )
                            .map(|event| Message::Markdown(event)),
                        )
                        .padding(10);

                        if !msg.is_reply {
                            right(text.style(styles::message))
                                .align_y(Vertical::Center)
                                .into()
                        } else {
                            text.into()
                        }
                    })
                    .collect();

                if !self.replying_string.content.is_empty() {
                    let text = container(
                        markdown::view_with(
                            self.replying_string.markdown.items(),
                            Theme::GruvboxLight,
                            &viewers::MarkdownViewer {},
                        )
                        .map(|event| Message::Markdown(event)),
                    )
                    .padding(10);
                    messages.push(text.into())
                }

                container(scrollable(column(messages).spacing(10).padding(20)).id("conversation"))
            } else {
                container(
                    text("Select a conversation or begin a new one.")
                        .font(NOTO_SANS)
                        .size(24)
                        .color(styles::text_color()),
                )
                .center_y(Length::Fill)
                .center_x(Length::Fill)
            }
        } else {
            container(
                text("Type and hit enter to begin a conversation.")
                    .font(NOTO_SANS)
                    .size(24)
                    .color(styles::text_color()),
            )
            .center_y(Length::Fill)
            .center_x(Length::Fill)
        };

        let conversation = row![
            Space::new().width(Length::FillPortion(1)),
            conversation
                .height(Length::FillPortion(6))
                .width(Length::FillPortion(4)),
            Space::new().width(Length::FillPortion(1))
        ];

        //
        // Input Field
        //
        //
        let text_editor_field = container(
            container(
                text_editor(&self.input)
                    .id("input")
                    .placeholder("Type something...")
                    .size(16)
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
            .style(styles::text_editor_container)
            .align_y(Vertical::Top)
            .padding(6),
        )
        .padding(20)
        .align_y(Vertical::Top)
        .width(Length::FillPortion(3))
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
        let messaging_area = container(
            column![
                Space::new().width(Length::Fill).height(60),
                conversation,
                input_field
            ]
            .align_x(Horizontal::Center),
        )
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
            text("Would you like to delete the conversation?")
                .color(styles::text_color())
                .font(NOTO_SANS)
                .size(16),
        )
        .title("Delete Chat")
        .push_button(
            iced_dialog::button("Delete", Message::DeleteChat(self.dialog_delete_chat))
                .style(styles::dialog_button),
        )
        .push_button(
            iced_dialog::button("Cancel", Message::DialogCancelDeleteChat)
                .style(styles::dialog_button),
        )
        .on_press(Message::DialogCancelDeleteChat)
        .into()
    }
}

fn reply_stream(
    server: Arc<Mutex<LlamaCpp>>,
    messages: Vec<CompletionMessage>,
) -> impl Sipper<Message, Message> {
    sipper(move |mut sender| async move {
        let mut server = server.lock().await;
        let mut stream = server.stream_response::<String>(messages).pin();
        while let Some(token) = stream.sip().await {
            sender
                .send(Message::ReplyMode(Chatting::Token(token)))
                .await
        }
        sender.send(Message::ReplyMode(Chatting::Complete)).await;
        Message::ReplyMode(Chatting::Complete)
    })
}
