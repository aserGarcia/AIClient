use convo::screen::{Screen, conversation, loading};
use iced::{Size, Subscription, Task, time, window};
use std::time::Duration;
use tracing_subscriber::fmt;

const DEFUALT_REPO: &'static str = "Qwen/Qwen3-4B-GGUF";
const DEFAULT_MODEL: &'static str = "Qwen3-4B-Q5_K_M.gguf";

use std::path::PathBuf;

use iced::widget::{button, column, container, progress_bar, text};
use iced::{Element, Length};
use iced::task::{sipper, Sipper};
use futures::StreamExt;

const DOWNLOAD_URL: &str = "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/main/Qwen3-4B-Q5_K_M.gguf";

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .run()
}

struct App {
    download_state: DownloadState,
    progress: f32,
    status_message: String,
    downloaded_mb: f64,
    total_mb: f64,
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
enum Message {
    StartDownload,
    DownloadProgress(DownloadProgress),
    DownloadComplete(Result<PathBuf, String>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                download_state: DownloadState::Idle,
                progress: 0.0,
                status_message: "Ready to download".to_string(),
                downloaded_mb: 0.0,
                total_mb: 0.0,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::StartDownload => {
                self.download_state = DownloadState::Downloading;
                self.progress = 0.0;
                self.downloaded_mb = 0.0;
                self.total_mb = 0.0;
                self.status_message = "Starting download...".to_string();
                
                Task::run(
                    download_file(DOWNLOAD_URL),
                    Message::DownloadProgress
                )
            }
            Message::DownloadProgress(progress) => {
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
                Task::none()
            }
            Message::DownloadComplete(result) => {
                match result {
                    Ok(path) => {
                        self.download_state = DownloadState::Complete(path.clone());
                        self.progress = 1.0;
                        self.status_message = format!("Complete! Saved to: {}", path.display());
                    }
                    Err(e) => {
                        self.download_state = DownloadState::Error(e.clone());
                        self.status_message = format!("Error: {}", e);
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let title = text("File Downloader").size(32);
        
        let url_text = text(format!("URL: {}", DOWNLOAD_URL)).size(12);
        
        let status = text(&self.status_message).size(14);
        
        let bar = progress_bar(0.0..=1.0, self.progress);
        
        let percentage = text(format!("{:.1}%", self.progress * 100.0)).size(20);
        
        let download_button = match &self.download_state {
            DownloadState::Idle | DownloadState::Error(_) | DownloadState::Complete(_) => {
                button(text("Download"))
                    .on_press(Message::StartDownload)
                    .padding(10)
            }
            DownloadState::Downloading => {
                button(text("Downloading..."))
                    .padding(10)
            }
        };

        let content = column![
            title,
            url_text,
            status,
            bar,
            percentage,
            download_button,
        ]
        .spacing(20)
        .padding(40)
        .max_width(700);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn download_file(url: &'static str) -> impl Sipper<Result<PathBuf, String>, DownloadProgress> {
    sipper(move |mut sender| async move {
        let client = reqwest::Client::new();
        let response: reqwest::Response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to start download: {}", e)),
        };
        
        let total_size: u64 = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| "Could not determine cache directory".to_string())?;
        let download_dir = cache_dir.join("downloads");
        std::fs::create_dir_all(&download_dir)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        
        let file_name = url.split('/').last().unwrap_or("download.bin");
        let file_path = download_dir.join(file_name);
        let mut file = std::fs::File::create(&file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            
            std::io::Write::write_all(&mut file, &chunk)
                .map_err(|e| format!("Failed to write to file: {}", e))?;
            
            downloaded += chunk.len() as u64;
            
            let _ = sender.send(DownloadProgress {
                downloaded,
                total: total_size,
            }).await;
        }
        
        Ok(file_path)
    })
}

// #[tokio::main]
// async fn main() -> iced::Result {
//     use convo_core::adapters::huggingface::{HFAdapter, DownloadProgress};
//
//     let hf = HFAdapter::new(DEFUALT_REPO.to_string()).unwrap();
//     let p = DownloadProgress{current: 0, total: 0};
//
//     let filename = hf.download(DEFAULT_MODEL, p).await.unwrap();
//     println!("filename: {}", filename.display());
//     Ok(())
    // fmt::init();
    // iced::application(Convo::new, Convo::update, Convo::view)
    //     .title(Convo::title)
    //     .window(window::Settings {
    //         size: Size {
    //             width: 1500.0,
    //             height: 1000.0,
    //         },
    //         ..Default::default()
    //     })
    //     .font(include_bytes!("../fonts/chat-icons.ttf").as_slice())
    //     .font(include_bytes!("../fonts/AveriaSerifLibre-Regular.ttf").as_slice())
    //     .font(include_bytes!("../fonts/OpenSans-VariableFont_wdth,wght.ttf").as_slice())
    //     .subscription(Convo::subscription)
    //     .run()
//}

// Drives the dynamic state of the GUI
// struct Convo {
//     screen: Screen,
// }
//
// #[derive(Clone)]
// enum Message {
//     Loading(loading::Message),
//     Conversation(conversation::Message),
// }
//
// impl Convo {
//     fn new() -> (Self, Task<Message>) {
//         //if let Ok((conversation, task)) = conversation::Conversation::new() {
//         if let Ok((loading, task)) = loading::Loading::new() {
//             // (
//             //     Self {
//             //         screen: Screen::Conversation(conversation),
//             //     },
//             //     task.map(Message::Conversation),
//             // )
//             (
//                 Self {
//                     screen: Screen::Loading(loading),
//                 },
//                 task.map(Message::Loading),
//             )
//         } else {
//             panic!("Could not load conversation.")
//         }
//     }
//
//     fn title(&self) -> String {
//         "Convo".to_string()
//     }
//
//     fn update(&mut self, message: Message) -> Task<Message> {
//         match message {
//             Message::Loading(message) => {
//                 let loading = if let Screen::Loading(loading) = &mut self.screen {
//                     Some(loading)
//                 } else {
//                     None
//                 };
//                 let Some(loading) = loading else {
//                     return Task::none();
//                 };
//
//                 let action = loading.update(message);
//                 match action {
//                     loading::Action::None => return Task::none(),
//                     loading::Action::Run(task) => return task.map(Message::Loading),
//                 }
//             }
//             Message::Conversation(message) => {
//                 let conversation = if let Screen::Conversation(conversation) = &mut self.screen {
//                     Some(conversation)
//                 } else {
//                     None
//                 };
//
//                 let Some(conversation) = conversation else {
//                     return Task::none();
//                 };
//                 let action = conversation.update(message);
//
//                 match action {
//                     conversation::Action::None => return Task::none(),
//                     conversation::Action::Run(task) => return task.map(Message::Conversation),
//                 }
//             }
//         };
//     }
//
//     fn view(&self) -> iced::Element<'_, Message> {
//         match &self.screen {
//             Screen::Loading(loading) => loading.view().map(Message::Loading),
//             Screen::Conversation(conversation) => conversation.view().map(Message::Conversation),
//         }
//     }
//
//     fn subscription(&self) -> Subscription<Message> {
//         time::every(Duration::from_secs(2))
//             .map(|_| Message::Conversation(conversation::Message::AutoSave))
//     }
// }
