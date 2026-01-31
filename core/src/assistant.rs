use crate::{MODEL_NAME, directory};
use std::sync::mpsc;
use thiserror::Error;
use tracing::{debug, error};

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::{LlamaModelParams, LlamaSplitMode};
use llama_cpp_2::sampling::LlamaSampler;

const BATCH_SIZE: usize = 512;
const MODEL_CONTEXT_SIZE: usize = 4096;

pub struct LlamaCpp {
    pub backend: LlamaBackend,
    pub model: LlamaModel,
    sampler: LlamaSampler,
}

#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Loading error {0}")]
    LoadError(String),

    #[error("Generation error {0}")]
    GenerationError(String),
}

pub struct GenerationRequest {
    pub input: String,
    pub response_tx: mpsc::Sender<Chatting>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Chatting {
    Token(String),
    Complete,
    Error(String),
}

impl LlamaCpp {
    pub fn load() -> Result<LlamaCpp, LlmError> {
        let backend = LlamaBackend::init().map_err(|e| LlmError::LoadError(e.to_string()))?;

        // Detecting devices
        let devices = llama_cpp_2::list_llama_ggml_backend_devices();
        for (i, dev) in devices.iter().enumerate() {
            debug!("Device {i:>2}: {}", dev.name);
            debug!("           Description: {}", dev.description);
            debug!("           Device Type: {:?}", dev.device_type);
            debug!("           Backend: {}", dev.backend);
            debug!(
                "           Memory total: {:?} MiB",
                dev.memory_total / 1024 / 1024
            );
            debug!(
                "           Memory free:  {:?} MiB",
                dev.memory_free / 1024 / 1024
            );
        }

        let model_params = {
            if cfg!(feature = "cuda") {
                LlamaModelParams::default()
                    .with_n_gpu_layers(1000)
                    .with_main_gpu(0)
                    .with_split_mode(LlamaSplitMode::Layer)
            } else {
                LlamaModelParams::default()
            }
        };

        // Load the model
        let model_path = directory::cache().join("downloads").join(MODEL_NAME);
        println!("Loading model from: {}", model_path.display());
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| LlmError::LoadError(e.to_string()))?;

        let sampler =
            LlamaSampler::chain_simple([LlamaSampler::dist(424242), LlamaSampler::greedy()]);

        Ok(LlamaCpp {
            backend,
            model,
            sampler,
        })
    }

    pub fn process_generation(&mut self, request: GenerationRequest) {
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(MODEL_CONTEXT_SIZE as u32)) // Context size
            .with_n_batch(BATCH_SIZE as u32) // Batch size
            .with_n_threads(num_threads as i32); // Number of threads

        // Create context
        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| LlmError::LoadError(e.to_string()))
            .unwrap();

        // Phi-3 chat template
        let system = "You are a helpful assistant, respond concisely.";
        let prompt = format!(
            "<|user|>Y{}\n{}<|end|>\n<|assistant|>\n",
            system, request.input
        );

        // Tokenize the prompt
        let tokens = match self
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
        let mut batch = LlamaBatch::new(BATCH_SIZE, 1);

        let last_index: i32 = (tokens.len() - 1) as i32;
        for (i, token) in tokens.iter().enumerate() {
            let is_last = i as i32 == last_index;
            if let Err(e) = batch.add(*token, i as i32, &[0], is_last) {
                let _ = request.response_tx.send(Chatting::Error(e.to_string()));
                return;
            };
        }

        if let Err(e) = &ctx.decode(&mut batch) {
            let _ = request.response_tx.send(Chatting::Error(e.to_string()));
            return;
        };

        // Generation parameters
        let mut n_cur = batch.n_tokens();
        let mut generated_tokens = Vec::new();

        debug!("\nGenerating response:\n");

        loop {
            let new_token = self.sampler.sample(&ctx, batch.n_tokens() - 1);
            self.sampler.accept(new_token);

            // Check for EOS token
            if self.model.is_eog_token(new_token) {
                debug!("\nEOS token reached");
                break;
            }

            generated_tokens.push(new_token);

            // Decode and print the token
            let token_str = match self
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

            if let Err(e) = &ctx.decode(&mut batch) {
                let _ = request.response_tx.send(Chatting::Error(e.to_string()));
                return;
            };
            n_cur += 1;
        }

        debug!("\n\nGeneration complete!");

        let _ = request.response_tx.send(Chatting::Complete);
        return;
    }
}
