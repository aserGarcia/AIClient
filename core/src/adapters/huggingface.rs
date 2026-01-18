use hf_hub::api::tokio::{ApiBuilder, ApiError, ApiRepo, Progress};
use std::path::PathBuf;

#[derive(Clone)]
pub struct DownloadProgress {
    pub current: usize,
    pub total: usize,
}

impl Progress for DownloadProgress {
    async fn init(&mut self, size: usize, _filename: &str) {
        self.total = size;
        self.current = 0;
    }

    async fn update(&mut self, size: usize) {
        self.current = size;
    }

    async fn finish(&mut self) {
        println!("Done downloading");
    }
}

#[derive(Debug)]
pub struct HFAdapter {
    pub api: ApiRepo,
    pub model: String,
}

impl HFAdapter {
    pub fn new(model_path: String) -> Result<Self, ApiError> {
        let repo = ApiBuilder::new().with_progress(true).build()?;
        let api = repo.model(model_path.clone());
        Ok(Self {
            api: api,
            model: model_path,
        })
    }

    pub async fn download(
        &self,
        filename: &str,
        progress: DownloadProgress,
    ) -> Result<PathBuf, ApiError> {
        let filename = self.api.download_with_progress(filename, progress).await?;
        Ok(filename)
    }
}
