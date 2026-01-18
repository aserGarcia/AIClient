use convo_core::adapters::huggingface::DownloadProgress;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Space, column, container, progress_bar, row, text};
use iced::{Background, Border, Font, Length, Task, Theme, color};
use thiserror::Error;

const AVERIA_SERIF_LIBRE: Font = Font::with_name("Averia Serif Libre");

pub struct Loading {
    progress: DownloadProgress,
}

#[derive(Clone)]
pub enum Message {
    Loading,
}

pub enum Action {
    None,
    Run(Task<Message>),
}

#[derive(Error, Debug)]
pub enum LoadingError {
    #[error("Failed to boot {0}")]
    Loading(String),
}

impl Loading {
    pub fn new() -> Result<(Self, Task<Message>), LoadingError> {
        Ok((
            Self {
                progress: DownloadProgress {
                    current: 0,
                    total: 100,
                },
            },
            Task::done(Message::Loading),
        ))
    }

    pub fn update(&self, message: Message) -> Action {
        match message {
            Message::Loading => {
                return Action::None;
            }
        }
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        container(
            column![
                container(text("Convo").font(AVERIA_SERIF_LIBRE).size(64)),
                row![
                    Space::new().width(Length::FillPortion(1)),
                    progress_bar(
                        0.0..=self.progress.total as f32,
                        self.progress.current as f32,
                    )
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
