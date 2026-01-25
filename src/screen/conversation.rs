use iced::alignment::{Horizontal, Vertical};
use iced::task::{Sipper, sipper};
use iced::widget::{
    Space, button, column, container, operation, row, scrollable, text, text_editor,
};
use iced::{Element, Font, Length, Task};
use iced_dialog::dialog;

use thiserror::Error;
use uuid::Uuid;

use tracing::{debug, error};

use crate::styles::styles;
use convo_core::{
    assistant::LlamaCpp,
    chat::{Chat, ChatMessage},
    db,
};

use std::io::Write;
use std::sync::mpsc;
use std::thread;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::sampling::LlamaSampler;

const CHAT_FONT: Font = Font::with_name("chat-icons");
const MOOLI: Font = Font::with_name("Mooli");
const NOTO_SANS: Font = Font::with_name("Noto Sans");

pub struct Conversation {
    model_tx: mpsc::Sender<GenerationRequest>,
    input: text_editor::Content,
    replying_string: String,
    chats: Vec<Chat>,
    db: db::Database,
    dialog_delete_chat_open: bool,
    dialog_delete_chat: Option<Uuid>,
    current_chat_id: Option<Uuid>,
}

#[derive(Clone)]
pub enum Message {
    Initialize,
    FocusInput,
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

#[derive(Debug, Clone, PartialEq)]
pub enum Chatting {
    Token(String),
    Complete,
    Error(String),
}

pub enum Action {
    None,
    Run(Task<Message>),
}

struct GenerationRequest {
    input: String,
    response_tx: mpsc::Sender<Chatting>,
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

        let (model_tx, model_rx) = mpsc::channel::<GenerationRequest>();

        let chats = db
            .load_chats()
            .map_err(|e| ConversationError::Loading(e.to_string()))?;
        debug!("chats loaded {}", chats.len());

        debug!("Loading model");
        thread::spawn(move || {
            let model = match LlamaCpp::load() {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to load model: {}", e);
                    return;
                }
            };
            while let Ok(request) = model_rx.recv() {
                process_generation(&model, request)
            }
        });

        Ok((
            Self {
                model_tx,
                input: text_editor::Content::new(),
                replying_string: String::new(),
                chats,
                db,
                dialog_delete_chat_open: false,
                dialog_delete_chat: None,
                current_chat_id: None,
            },
            Task::done(Message::Initialize),
        ))
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
                let uuid = Uuid::new_v4();
                let chat = Chat {
                    id: uuid,
                    title: format!("Chat {:.8}", uuid.to_string()),
                    minor_text: String::new(),
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
                                is_reply: false,
                            };
                            self.chats[idx].messages.push(message);
                        }
                    } else {
                        let id = Uuid::new_v4();
                        self.current_chat_id = Some(id);
                        let message = ChatMessage {
                            id: 0,
                            chat_id: id,
                            content: self.input.text(),
                            is_reply: false,
                        };
                        self.chats.push(Chat {
                            id,
                            title: format!("Chat {:.8}", id.to_string()),
                            minor_text: format!("{:.15}...", self.input.text()),
                            messages: vec![message],
                        });
                    }

                    self.db.needs_save = true;
                    let input = self.input.text().clone();
                    self.input = text_editor::Content::new();

                    let model_tx = self.model_tx.clone();
                    return Action::Run(Task::stream(generate_reply_with_worker(model_tx, input)));
                }
                return Action::None;
            }
            Message::ReplyMode(message) => {
                match message {
                    Chatting::Token(tok) => {
                        self.replying_string.push_str(tok.as_str());
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
                                    content: self.replying_string.clone(),
                                    is_reply: true,
                                };
                                self.chats[idx].messages.push(message);

                                self.replying_string.clear();
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

                let mut chat_button = button(container(column![
                    text(chat.title.clone()).font(NOTO_SANS).size(16),
                    text(chat.minor_text.clone()).font(NOTO_SANS).size(12)
                ]))
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
                            text(msg.content.clone())
                                .color(styles::text_color())
                                .font(NOTO_SANS)
                                .size(16),
                        )
                        .padding(10);
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

                if !self.replying_string.is_empty() {
                    let text = container(
                        text(self.replying_string.clone())
                            .color(styles::text_color())
                            .font(NOTO_SANS)
                            .size(16),
                    )
                    .padding(10);
                    let bubble =
                        row![text, Space::new().width(Length::Fill)].align_y(Vertical::Center);
                    messages.push(bubble.into())
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

        let conversation = conversation.height(Length::FillPortion(6)).max_width(800);

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

fn process_generation(model: &LlamaCpp, request: GenerationRequest) {
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(std::num::NonZeroU32::new(4096)) // Context size
        .with_n_batch(512) // Batch size
        .with_n_threads(4); // Number of threads

    // Create context
    let mut ctx = match model.model.new_context(&model.backend, ctx_params) {
        Ok(c) => c,
        Err(e) => {
            let _ = request.response_tx.send(Chatting::Error(e.to_string()));
            return;
        }
    };

    // Phi-3 chat template
    let prompt = format!("<|user|>\n{}<|end|>\n<|assistant|>\n", request.input);

    // Tokenize the prompt
    let tokens = match model
        .model
        .str_to_token(&prompt, llama_cpp_2::model::AddBos::Always)
    {
        Ok(t) => t,
        Err(e) => {
            let _ = request.response_tx.send(Chatting::Error(e.to_string()));
            return;
        }
    };

    debug!("Tokenized {} tokens", tokens.len());

    // Decode the initial prompt
    let mut batch = LlamaBatch::new(512, 1);

    let last_index: i32 = (tokens.len() - 1) as i32;
    for (i, token) in tokens.iter().enumerate() {
        let is_last = i as i32 == last_index;
        if let Err(e) = batch.add(*token, i as i32, &[0], is_last) {
            let _ = request.response_tx.send(Chatting::Error(e.to_string()));
            return;
        };
    }

    if let Err(e) = ctx.decode(&mut batch) {
        let _ = request.response_tx.send(Chatting::Error(e.to_string()));
        return;
    };

    // Generation parameters
    let max_tokens = 100;
    let mut n_cur = batch.n_tokens();
    let mut generated_tokens = Vec::new();

    debug!("\nGenerating response:\n");

    let mut sampler =
        LlamaSampler::chain_simple([LlamaSampler::dist(424242), LlamaSampler::greedy()]);

    for _ in 0..max_tokens {
        let new_token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(new_token);

        // Check for EOS token
        if model.model.is_eog_token(new_token) {
            debug!("\nEOS token reached");
            break;
        }

        generated_tokens.push(new_token);

        // Decode and print the token
        let token_str = match model
            .model
            .token_to_str(new_token, llama_cpp_2::model::Special::Tokenize)
        {
            Ok(s) => s,
            Err(e) => {
                let _ = request.response_tx.send(Chatting::Error(e.to_string()));
                return;
            }
        };

        let _ = request.response_tx.send(Chatting::Token(token_str));

        // Prepare next batch with just the new token
        batch.clear();
        if let Err(e) = batch.add(new_token, n_cur, &[0], true) {
            let _ = request.response_tx.send(Chatting::Error(e.to_string()));
            return;
        };

        if let Err(e) = ctx.decode(&mut batch) {
            let _ = request.response_tx.send(Chatting::Error(e.to_string()));
            return;
        };
        n_cur += 1;
    }

    debug!("\n\nGeneration complete!");

    let _ = request.response_tx.send(Chatting::Complete);
    return;
}

fn generate_reply_with_worker(
    model_tx: mpsc::Sender<GenerationRequest>,
    input: String,
) -> impl Sipper<Message, Message> {
    sipper(move |mut sender| async move {
        let (response_tx, response_rx) = mpsc::channel();

        let request = GenerationRequest { input, response_tx };

        if model_tx.send(request).is_err() {
            return Message::ReplyMode(Chatting::Error("Model worker died".to_string()));
        }

        while let Ok(chatting) = response_rx.recv() {
            let is_complete = chatting == Chatting::Complete;
            let _ = sender.send(Message::ReplyMode(chatting)).await;
            if is_complete {
                break;
            }
        }

        Message::ReplyMode(Chatting::Complete)
    })
}
