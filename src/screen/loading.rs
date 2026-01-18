use convo_core::adapters::huggingface::DownloadProgress;
use iced::widget::progress_bar;

pub struct Loading {
  progress: DownloadProgress
}

enum Message {
  
}

impl Loading {
  
  pub fn new() -> Self {
    Self {
      progress: DownloadProgress{0,0}
    }
  }

  pub fn update() {}

  pub fn view(&self) {
    container(
      progress_bar(0.0..=self.progress.total, self.progress.current)
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .align_y(Vertical::Center)
    .into()
  }
}
