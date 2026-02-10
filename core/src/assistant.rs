use crate::chat::ChatMessage;
use crate::{MODEL_NAME, MODEL_REPO_PATH, directory};
use async_openai::Client;
use async_openai::config::{Config, OpenAIConfig};
use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessage as AssistantMessage, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage as SystemMessage,
    ChatCompletionRequestUserMessage as UserMessage, CreateChatCompletionRequest,
    CreateChatCompletionRequestArgs,
};
use futures::StreamExt;
use sipper::{Sipper, sipper};
use std::process::Stdio;
use thiserror::Error;
use tokio::process;
use tokio::time::{self, Duration};
use tracing::{debug, error};

const PORT: usize = 8081;
const HOST: &str = "127.0.0.1";

#[derive(Debug)]
pub struct LlamaCpp {
    process: process::Child,
    pub client: Client<OpenAIConfig>,
    pub chat_completion_request: CreateChatCompletionRequest,
}

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Loading error {0}")]
    LoadError(String),

    #[error("Generation error {0}")]
    GenerationError(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Chatting {
    Token(String),
    Complete,
    Error(String),
}

impl LlamaCpp {
    pub fn url(&self) -> String {
        self.client.config().url("")
    }

    pub async fn boot() -> Result<LlamaCpp, LlmError> {
        // TODO: switch based off backed and OS
        let executable = "./servers/llama-cpu-ubuntu-x64/llama-server";
        debug!("Starting child process");
        let child_process = process::Command::new(executable)
            .args(
                format!(
                    "-hf {model_repo} --host {host} --port {port}",
                    model_repo = MODEL_REPO_PATH,
                    host = HOST,
                    port = PORT
                )
                .split_whitespace(),
            )
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| LlmError::LoadError(e.to_string()))?;

        let config = OpenAIConfig::new().with_api_base(format!("http://{}:{}", HOST, PORT));
        let client = Client::with_config(config);

        let request = CreateChatCompletionRequestArgs::default()
            .model("microsoft/Phi-3-mini-4k-instruct-gguf:Phi-3-mini-4k-instruct-q4.gguf")
            .n(1)
            .stream(true)
            .seed(424242)
            .build()
            .map_err(|e| LlmError::LoadError(e.to_string()))?;

        Ok(LlamaCpp {
            process: child_process,
            client: client,
            chat_completion_request: request,
        })
    }

    pub async fn wait_until_ready(&mut self) -> Result<(), LlmError> {
        loop {
            if let Some(status) = self
                .process
                .try_wait()
                .map_err(|e| LlmError::LoadError(e.to_string()))?
            {
                return Err(LlmError::LoadError(format!(
                    "llama-server exited unexpectedly: {status}"
                )));
            }

            if let Ok(response) = reqwest::Client::new()
                .get(format!("{}/health", self.url()))
                .send()
                .await
                && response.error_for_status().is_ok()
            {
                println!("Server loaded, is healthy");
                break;
            }

            time::sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }

    pub fn stream_response<T>(&mut self, messages: Vec<ChatMessage>) -> impl Sipper<T, T>
    where
        T: From<String>,
    {
        let mut chat_completion_messages: Vec<ChatCompletionRequestMessage> =
            vec![SystemMessage::from("You are a helpful assistant.").into()];

        chat_completion_messages.extend(messages.iter().map(|m| {
            if m.is_reply {
                AssistantMessage::from(m.content.clone()).into()
            } else {
                UserMessage::from(m.content.clone()).into()
            }
        }));

        self.chat_completion_request.messages = chat_completion_messages;

        sipper(|mut sender| async move {
            let mut stream = self
                .client
                .chat()
                .create_stream(self.chat_completion_request.clone())
                .await
                .expect("Stream not created");

            while let Some(resp) = stream.next().await {
                match resp {
                    Ok(ccr) => {
                        if let Some(content) = ccr.choices[0].delta.content.as_ref() {
                            sender.send(T::from(content.to_owned())).await;
                        }
                    }
                    Err(e) => {
                        sender.send(T::from(e.to_string())).await;
                    }
                }
            }
            T::from(String::from("Done"))
        })
    }
}
