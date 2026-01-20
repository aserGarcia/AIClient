use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, column, container, progress_bar, row, text};
use iced::{Background, Border, Font, Length, Task, Theme, color};
use iced::task::{sipper, Sipper};
use futures::StreamExt;
use std::path::PathBuf;
use std::time::Duration;
use convo_core::directory;

const AVERIA_SERIF_LIBRE: Font = Font::with_name("Averia Serif Libre");
const DOWNLOAD_URL: &str = "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/main/Qwen3-4B-Q5_K_M.gguf";

pub struct Loading {
    download_state: DownloadState,
    progress: f32,
    status_message: String,
    downloaded_mb: f64,
    total_mb: f64
}

#[derive(Debug, Clone)]
enum DownloadState {
    Idle,
    Downloading,
    Complete(PathBuf),
    Error(String),
}

#[derive(Debug, Clone)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

#[derive(Debug, Clone)]
pub enum Message {
    StartDownload,
    DownloadUpdate(DownloadUpdate),
}

#[derive(Debug, Clone)]
enum DownloadUpdate {
    Progress(DownloadProgress),
    Complete(Result<PathBuf, String>),
    Error(String)
}

pub enum Action {
    None,
    Run(Task<Message>),
    Continue
}


impl Loading {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                download_state: DownloadState::Idle,
                progress: 0.0,
                status_message: "Ready to download".to_string(),
                downloaded_mb: 0.0,
                total_mb: 0.0,
            },
            Task::done(Message::StartDownload),
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::StartDownload => {
                self.download_state = DownloadState::Downloading;
                self.progress = 0.0;
                self.downloaded_mb = 0.0;
                self.total_mb = 0.0;
                self.status_message = "Starting download...".to_string();
                                
                Action::Run(Task::stream(download_file(DOWNLOAD_URL)))
            }
            Message::DownloadUpdate(update) => {
                match update {
                    DownloadUpdate::Progress(progress) => {
                        self.downloaded_mb = progress.downloaded as f64 / 1_048_576.0;
                        self.total_mb = progress.total as f64 / 1_048_576.0;
                        self.progress = if progress.total > 0 {
                            progress.downloaded as f32 / progress.total as f32
                        } else {
                            0.0
                        };
                        self.status_message = format!(
                            "Downloading... {:.2} MB / {:.2} MB", 
                            self.downloaded_mb, 
                            self.total_mb
                        );
                        return Action::None;
                    }
                    DownloadUpdate::Complete(result) => {
                        match result {
                            Ok(path) => {
                                self.download_state = DownloadState::Complete(path.clone());
                                self.progress = 1.0;
                                self.status_message = format!("Complete! Saved to: {}", path.display());
                                println!("{}", self.status_message);
                                return Action::Continue;
                            }
                            Err(e) => {
                                self.download_state = DownloadState::Error(e.clone());
                                self.status_message = format!("Error: {}", e);
                                println!("{}", self.status_message);
                                return Action::Continue;
                            }
                        }
                    }
                    DownloadUpdate::Error(e) => {
                        println!("Error {}", e);
                        return Action::None
                    }
                }
            }
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        container(
            column![
                container(text("Convo").font(AVERIA_SERIF_LIBRE).size(64)),
                row![
                    Space::new().width(Length::FillPortion(1)),
                    progress_bar(0.0..=1.0, self.progress)
                    .style(|_theme: &Theme| progress_bar::Style {
                        background: Background::Color(color!(0xDBDBDB)),
                        bar: Background::Color(color!(0x0c0c0c)),
                        border: Border {
                            radius: 10.0.into(),
                            ..Default::default()
                        }
                    })
                    .length(Length::FillPortion(2))
                    .girth(Length::Fixed(10.0)),
                    Space::new().width(Length::FillPortion(1))
                ],
                Space::new().height(Length::Fixed(60.0))
            ]
            .align_x(Horizontal::Center),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(|_theme: &Theme| container::Style {
            background: Some(color!(0x242424).into()),
            ..Default::default()
        })
        .into()
    }
}


fn download_file(url: &'static str) -> impl Sipper<Message, Message> {
    sipper(move |mut sender| async move {
        let client = reqwest::Client::new();
        let response: reqwest::Response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => return Message::DownloadUpdate(DownloadUpdate::Error(e.to_string())),
        };
        
        let total_size: u64 = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        
        let cache_dir = directory::cache();
        let download_dir = cache_dir.join("downloads");
        std::fs::create_dir_all(&download_dir)
            .map_err(|e| Message::DownloadUpdate(
                    DownloadUpdate::Error(format!("Failed to create directory: {}", e))
                    ));
        
        let file_name = url.split('/').last().unwrap_or("download.bin");
        let file_path = download_dir.join(file_name);
        let mut file = match std::fs::File::create(&file_path) {
            Ok(f) => f,
            Err(e) => return Message::DownloadUpdate(
                    DownloadUpdate::Error(format!("Failed to create file: {}", e))),
        };
        
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => return Message::DownloadUpdate(
                    DownloadUpdate::Error(format!("Download error: {}", e))
                    ),
            };
            
            std::io::Write::write_all(&mut file, &chunk)
                .map_err(|e| Message::DownloadUpdate(
                        DownloadUpdate::Error(format!("Failed to write to file: {}", e))
                ));
            
            downloaded += chunk.len() as u64;
            let message = Message::DownloadUpdate(DownloadUpdate::Progress(DownloadProgress {
                downloaded,
                total: total_size,
            }));
            
            let _ = sender.send(message).await;
        }
        
        println!("Donwload complete, returning Ok");
        let msg = Message::DownloadUpdate(DownloadUpdate::Complete(Ok(file_path)));
        let _ = sender.send(msg.clone()).await;
        return msg
    })
}
