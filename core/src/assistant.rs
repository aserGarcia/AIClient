use crate::directory;
use std::io::Write;
use thiserror::Error;
use tracing::{debug, error, info};
use iced::task::{Sipper, sipper};
use convo::screen::conversation::{Message, Chatting};

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::sampling::LlamaSampler;

pub struct LlamaCpp {
    pub backend: LlamaBackend,
    pub model: LlamaModel,
}



#[derive(Error, Debug)]
pub enum LlmError {
    #[error("Loading error {0}")]
    LoadError(String),

    #[error("Generation error {0}")]
    GenerationError(String),
}

impl LlamaCpp {
    pub fn load() -> Result<LlamaCpp, LlmError> {
        let backend = LlamaBackend::init().map_err(|e| LlmError::LoadError(e.to_string()))?;

        // Model parameters
        let model_params = LlamaModelParams::default();

        // Load the model
        let model_path = directory::cache().join("downloads/Phi-3-mini-4k-instruct-q4.gguf");
        println!("Loading model from: {}", model_path.display());
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| LlmError::LoadError(e.to_string()))?;

        Ok(LlamaCpp { backend, model })
    }

    pub fn reply(&self, input: String) -> impl Sipper<Message, Message> {
        sipper(move |mut sender| async move {
            let ctx_params = LlamaContextParams::default()
                        .with_n_ctx(std::num::NonZeroU32::new(4096)) // Context size
                        .with_n_batch(512) // Batch size
                        .with_n_threads(4); // Number of threads

            // Create context
            let mut ctx = match self
                .model
                .new_context(&self.backend, ctx_params) {
                    Ok(c) => c,
                    Err(e) => {return Message::ReplyMode(Chatting::Error(e.to_string()));}
                };

            // Phi-3 chat template
            let prompt = format!("<|user|>\n{}<|end|>\n<|assistant|>\n", input);

            // Tokenize the prompt
            let tokens = match self
                .model
                .str_to_token(&prompt, llama_cpp_2::model::AddBos::Always) {
                    Ok(t) => t,
                    Err(e) => {return Message::ReplyMode(Chatting::Error(e.to_string()));}
                };

            debug!("Tokenized {} tokens", tokens.len());

            // Decode the initial prompt
            let mut batch = LlamaBatch::new(512, 1);

            let last_index: i32 = (tokens.len() - 1) as i32;
            for (i, token) in tokens.iter().enumerate() {
                let is_last = i as i32 == last_index;
                if let Err(e) = batch
                    .add(*token, i as i32, &[0], is_last) {
                        return Message::ReplyMode(Chatting::Error(e.to_string()));
                    };
            }

            if let Err(e) = ctx.decode(&mut batch) {
                return Message::ReplyMode(Chatting::Error(e.to_string()));
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
                if self.model.is_eog_token(new_token) {
                    debug!("\nEOS token reached");
                    break;
                }

                generated_tokens.push(new_token);

                // Decode and print the token
                let token_str = match self
                    .model
                    .token_to_str(new_token, llama_cpp_2::model::Special::Tokenize) {
                        Ok(s) => s,
                        Err(e) => {return Message::ReplyMode(Chatting::Error(e.to_string()));}
                    };

                
                let message = Message::ReplyMode(Chatting::Token(token_str));
                let _ = sender.send(message).await;

                // Prepare next batch with just the new token
                batch.clear();
                if let Err(e) = batch
                    .add(new_token, n_cur, &[0], true) {
                        return Message::ReplyMode(Chatting::Error(e.to_string()));
                    };

                if let Err(e) = ctx.decode(&mut batch) {
                    return Message::ReplyMode(Chatting::Error(e.to_string()));
                };
                n_cur += 1;
            }

            println!("\n\nGeneration complete!");

            let _  = sender.send(Message::ReplyMode(Chatting::Complete)).await;
            return Message::ReplyMode(Chatting::Complete);
        })
       
    }
}
