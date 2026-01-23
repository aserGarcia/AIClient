use crate::styles::styles;
use convo_core::directory;
use futures::StreamExt;
use iced::alignment::{Horizontal, Vertical};
use iced::task::{Sipper, sipper};
use iced::widget::{Space, column, container, progress_bar, row, text};
use iced::{Background, Border, Font, Length, Task, Theme, color};
use std::path::PathBuf;
use tracing::{debug, error};

const MOOLI: Font = Font::with_name("Mooli");
const DOWNLOAD_URL: &str =
    "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/main/Qwen3-4B-Q5_K_M.gguf";

pub struct Loading {
    progress: DownloadProgress,
}

#[derive(Debug, Clone)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

impl DownloadProgress {
    fn get_progress(&self) -> f32 {
        if self.total > 0 {
            self.downloaded as f32 / self.total as f32
        } else {
            0.0
        }
    }
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
    Error(String),
}

pub enum Action {
    None,
    Run(Task<Message>),
    Error(String),
    Continue,
}

impl Loading {
    pub fn new() -> (Self, Task<Message>) {
        debug!("Ready to download");
        (
            Self {
                progress: DownloadProgress {
                    downloaded: 0,
                    total: 0,
                },
            },
            Task::done(Message::StartDownload),
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::StartDownload => {
                self.progress.downloaded = 0;
                debug!("Starting download...");

                Action::Run(Task::stream(download_file(DOWNLOAD_URL)))
            }
            Message::DownloadUpdate(update) => match update {
                DownloadUpdate::Progress(progress) => {
                    self.progress = progress;
                    return Action::None;
                }
                DownloadUpdate::Complete(result) => match result {
                    Ok(path) => {
                        self.progress.downloaded = self.progress.total;
                        debug!("Complete! Saved to: {}", path.display());
                        return Action::Continue;
                    }
                    Err(e) => {
                        error!("Error: {}", e);
                        return Action::Error(e);
                    }
                },
                DownloadUpdate::Error(e) => {
                    error!("Error {}", e);
                    return Action::Error(e.to_string());
                }
            },
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        container(
            column![
                container(
                    text("Convo")
                        .color(styles::text_color())
                        .font(MOOLI)
                        .size(64)
                ),
                row![
                    Space::new().width(Length::FillPortion(1)),
                    progress_bar(0.0..=1.0, self.progress.get_progress())
                        .style(|_theme: &Theme| progress_bar::Style {
                            background: Background::Color(styles::background_color()),
                            bar: Background::Color(styles::text_color()),
                            border: Border {
                                color: styles::border_color(),
                                radius: 10.0.into(),
                                width: 1.0.into()
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
            background: Some(styles::background_color().into()),
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
        match std::fs::create_dir_all(&download_dir) {
            Ok(()) => {}
            Err(e) => {
                let msg = Message::DownloadUpdate(DownloadUpdate::Error(format!(
                    "Failed to create directory: {}",
                    e
                )));
                let _ = sender.send(msg.clone()).await;
                return msg;
            }
        }

        let file_name = url.split('/').last().unwrap_or("download.bin");
        let file_path = download_dir.join(file_name);

        if !file_path.exists() {
            let mut file = match std::fs::File::create(&file_path) {
                Ok(f) => f,
                Err(e) => {
                    let msg = Message::DownloadUpdate(DownloadUpdate::Error(format!(
                        "Failed to create file: {}",
                        e
                    )));
                    let _ = sender.send(msg.clone()).await;
                    return msg;
                }
            };

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        let msg = Message::DownloadUpdate(DownloadUpdate::Error(format!(
                            "Download Error: {}",
                            e
                        )));
                        let _ = sender.send(msg.clone()).await;
                        return msg;
                    }
                };

                match std::io::Write::write_all(&mut file, &chunk) {
                    Ok(()) => {}
                    Err(e) => {
                        let msg = Message::DownloadUpdate(DownloadUpdate::Error(format!(
                            "Download Error: {}",
                            e
                        )));
                        let _ = sender.send(msg.clone()).await;
                        return msg;
                    }
                }

                downloaded += chunk.len() as u64;
                let message = Message::DownloadUpdate(DownloadUpdate::Progress(DownloadProgress {
                    downloaded,
                    total: total_size,
                }));

                let _ = sender.send(message).await;
            }
        }

        println!("Donwload complete, returning Ok");
        let msg = Message::DownloadUpdate(DownloadUpdate::Complete(Ok(file_path)));
        let _ = sender.send(msg.clone()).await;
        return msg;
    })
}
