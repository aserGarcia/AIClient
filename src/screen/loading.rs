use crate::styles::styles;
use convo_core::{DOWNLOAD_URL, directory};
use futures::StreamExt;
use iced::alignment::Horizontal;
use iced::task::{Sipper, sipper};
use iced::widget::{Space, column, container, progress_bar, row, text};
use iced::{Background, Border, Font, Length, Task, Theme};
use std::path::PathBuf;
use tracing::{debug, error, info};

const MOOLI: Font = Font::with_name("Mooli");
const PROGRESS_BAR_HEIGHT: f32 = 10.0;

pub struct Loading {
    progress: DownloadProgress,
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

impl DownloadProgress {
    fn new() -> Self {
        Self {
            downloaded: 0,
            total: 0,
        }
    }

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
pub enum DownloadUpdate {
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
                progress: DownloadProgress::new(),
            },
            Task::done(Message::StartDownload),
        )
    }

    pub fn update(&mut self, message: Message) -> Action {
        match message {
            Message::StartDownload => {
                self.progress = DownloadProgress::new();
                debug!("Starting download...");
                Action::Run(Task::stream(download_file(DOWNLOAD_URL)))
            }
            Message::DownloadUpdate(update) => match update {
                DownloadUpdate::Progress(progress) => {
                    self.progress = progress;
                    Action::None
                }
                DownloadUpdate::Complete(result) => match result {
                    Ok(path) => {
                        self.progress.downloaded = self.progress.total;
                        info!("Download complete! Saved to: {}", path.display());
                        Action::Continue
                    }
                    Err(e) => {
                        error!("Download failed: {}", e);
                        Action::Error(e)
                    }
                },
                DownloadUpdate::Error(e) => {
                    error!("Download error: {}", e);
                    Action::Error(e)
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
                        .girth(Length::Fixed(PROGRESS_BAR_HEIGHT)),
                    Space::new().width(Length::FillPortion(1))
                ],
            ]
            .align_x(Horizontal::Center),
        )
        .center(Length::Fill)
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
            Ok(()) => {
                info!("Downloads directory exists");
            }
            Err(e) => {
                error!("Failed to create downloads dir");
                let msg = Message::DownloadUpdate(DownloadUpdate::Error(format!(
                    "Failed to create directory: {}",
                    e
                )));
                sender.send(msg.clone()).await;
                return msg;
            }
        }

        let file_name = url.split('/').last().unwrap_or("download.bin");
        let file_path = download_dir.join(file_name);

        if !file_path.exists() {
            let mut file = match std::fs::File::create(&file_path) {
                Ok(f) => {
                    debug!("Creating filepath to download");
                    f
                }
                Err(e) => {
                    error!("Failed to create file {}", file_path.display());
                    let msg = Message::DownloadUpdate(DownloadUpdate::Error(format!(
                        "Failed to create file: {}",
                        e
                    )));
                    sender.send(msg.clone()).await;
                    return msg;
                }
            };

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Stream error {e}");
                        let msg = Message::DownloadUpdate(DownloadUpdate::Error(format!(
                            "Download Error: {}",
                            e
                        )));
                        sender.send(msg.clone()).await;
                        return msg;
                    }
                };

                match std::io::Write::write_all(&mut file, &chunk) {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Download error {e}");
                        let msg = Message::DownloadUpdate(DownloadUpdate::Error(format!(
                            "Download Error: {}",
                            e
                        )));
                        sender.send(msg.clone()).await;
                        return msg;
                    }
                }

                downloaded += chunk.len() as u64;
                let message = Message::DownloadUpdate(DownloadUpdate::Progress(DownloadProgress {
                    downloaded,
                    total: total_size,
                }));

                sender.send(message).await;
            }
        }
        // updating the progress bar
        let complete_progressbar =
            Message::DownloadUpdate(DownloadUpdate::Progress(DownloadProgress {
                downloaded: total_size,
                total: total_size,
            }));
        sender.send(complete_progressbar).await;

        info!("Download complete for {}, returning Ok", file_name);

        let msg = Message::DownloadUpdate(DownloadUpdate::Complete(Ok(
            directory::cache().to_path_buf()
        )));
        sender.send(msg.clone()).await;
        return msg;
    })
}
