use crate::adapters::huggingface::HFAdapter;
use hf_hub::api::tokio::ApiError;
use std::path::PathBuf;

const DEFUALT_REPO: &'static str = "Qwen/Qwen3-4B-GGUF";
const DEFAULT_MODEL: &'static str = "Qwen3-4B-Q5_K_M.gguf";

// pub fn get_model() -> Result<PathBuf, ApiError> {
//   let hf = HFAdapter::new(DEFAULT_REPO);
//
//   let local_filename = hf.api.download_with_progress()
// }
