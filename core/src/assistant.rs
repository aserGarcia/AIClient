use crate::directory;
use thiserror::Error;
use tracing::{debug, error};

use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::model::params::{LlamaModelParams, LlamaSplitMode};

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
                    .with_split_mode(LlamaSplitMode::None)
            } else {
                LlamaModelParams::default()
            }
        };

        // Load the model
        let model_path = directory::cache().join("downloads/Phi-3-mini-4k-instruct-q4.gguf");
        println!("Loading model from: {}", model_path.display());
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| LlmError::LoadError(e.to_string()))?;

        Ok(LlamaCpp { backend, model })
    }
}
