use crate::{MODEL_NAME, MODEL_REPO_PATH, directory};
use reqwest;
use std::process::Stdio;
use thiserror::Error;
use tokio::process;
use tokio::time::{self, Duration};
use tracing::{debug, error};

const BATCH_SIZE: usize = 512;
const MODEL_CONTEXT_SIZE: usize = 4096;

#[derive(Debug)]
pub struct LlamaCpp {
    model: String,
    host: String,
    port: usize,
    process: process::Child,
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
        format!("http://{}:{}", self.host, self.port)
    }

    pub async fn boot() -> Result<LlamaCpp, LlmError> {
        // TODO: switch based off backed and OS
        let executable = "./servers/llama-cpu-ubuntu-x64/llama-server";
        let port = 8081;
        let host = "127.0.0.1";
        let child_process = process::Command::new(executable)
            .args(
                format!(
                    "-hf {model_repo} --host {host} --port {port}",
                    model_repo = MODEL_REPO_PATH,
                    host = host,
                    port = port
                )
                .split_whitespace(),
            )
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| LlmError::LoadError(e.to_string()))?;

        Ok(LlamaCpp {
            model: MODEL_NAME.to_string(),
            host: host.to_string(),
            port: port,
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
