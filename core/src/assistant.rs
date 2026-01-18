use crate::adapters::huggingface::HFAdapter;
use std::path::PathBuf;
use hf_hub::api::tokio::ApiError;

const DEFUALT_REPO: String = String::from("Qwen/Qwen3-4B-GGUF");
const DEFAULT_MODEL: String = String::from("Qwen3-4B-Q5_K_M.gguf");

pub fn get_model() -> Result<PathBuf, ApiError> {
  let hf = HFAdapter::new(DEFAULT_REPO);

  let local_filename = hf.api.download_with_progress()
}
