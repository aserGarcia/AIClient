use crate::{MODEL_NAME, MODEL_REPO_PATH, directory};
use async_openai::Client;
use async_openai::config::{Config, OpenAIConfig};
use reqwest;
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

        Ok(LlamaCpp {
            client: client,
            process: child_process,
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
}
