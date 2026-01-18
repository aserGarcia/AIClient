use hf_hub::{Repo, RepoType, api::tokio::{ApiBuilder, ApiRepo, ApiError}};

#[drive(Clone)]
pub struct DownloadProgress {
    current: usize,
    total: usize
}

impl Progress for DownloadProgress {
    async pub fn init(&mut self, size: usize, _filename: &str) {
        self.total = size;
        self.current = 0;
    }

    async pub fn update(&mut self, size: usize) {
        self.current = size;
    }

    async pub fn finish(&mut self) {
        println!("Done downloading");
    }
}

#[derive(Debug)]
struct HFAdapter {
    pub api: ApiRepo,
    pub model: String 
}

impl HFAdapter {
    pub fn new(model_path: String) -> Self {
        let repo = ApiBuilder::new()
            .with_progress(true)
            .build();
        let api = api.model(model_path);
        Self {api, model_path}
    }

    pub fn download(filename: &str) -> Result<PathBuf, ApiError> {
        let progress = DonwloadProgress{current: 0, total: 0};
        let filename = self.api.download_with_progress(filename, progress).await?;
        Ok(filename)
    }
}
