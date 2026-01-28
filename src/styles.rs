pub mod styles {
    use iced::widget::{button, container, text_editor};
    use iced::{Border, Color, Theme, color};

    pub fn background_dark_color() -> Color {
        color!(0x030303)
    }

    pub fn background_color() -> Color {
        color!(0x0A0A0A)
    }

    pub fn background_light_color() -> Color {
        color!(0x171717)
    }

    pub fn text_color() -> Color {
        color!(0xF2F2F2)
    }

    pub fn text_color_muted() -> Color {
        color!(0xB0B0B0)
    }

    pub fn text_color_dark() -> Color {
        color!(0x5B5B5B)
    }

    pub fn border_color() -> Color {
        color!(0x474747)
    }

    pub fn primary_color() -> Color {
        color!(0xCBAD62)
    }

    pub fn secondary_color() -> Color {
        color!(0x97AFF2)
    }

    pub fn highlight_color() -> Color {
        color!(0x636363)
    }

    pub fn sidebar(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(text_color().into()),
            background: Some(background_color().into()),
            border: Border {
                color: border_color(),
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn messaging_area(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(background_color().into()),
            border: Border {
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn message(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(text_color().into()),
            background: Some(background_color().into()),
            border: Border {
                radius: 5.0.into(),
                color: border_color(),
                width: 1.0,
                ..Default::default()
            },

            ..Default::default()
        }
    }

    pub fn convo_header(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(background_dark_color().into()),
            border: Border {
                color: border_color(),
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn chat_container_default(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(background_color().into()),
            ..Default::default()
        }
    }

    pub fn chat_container_selected(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(Color::BLACK),
            background: Some(primary_color().into()),
            border: Border {
                color: border_color(),
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn new_chat_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered => button::Style {
                text_color: background_dark_color().into(),
                ..Default::default()
            },
            _ => button::Style {
                text_color: Color::WHITE,
                ..Default::default()
            },
        }
    }

    pub fn delete_chat_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered => button::Style {
                text_color: background_dark_color().into(),
                ..Default::default()
            },
            _ => button::Style {
                text_color: Color::WHITE,
                ..Default::default()
            },
        }
    }

    pub fn open_chat_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered => button::Style {
                text_color: background_dark_color(),
                background: Some(secondary_color().into()),
                ..Default::default()
            },
            _ => button::Style {
                text_color: text_color(),
                background: Some(background_color().into()),
                ..Default::default()
            },
        }
    }

    pub fn chat_selected(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered | button::Status::Active => button::Style {
                text_color: background_dark_color(),
                background: Some(primary_color().into()),
                ..Default::default()
            },
            _ => button::Style {
                text_color: text_color(),
                background: Some(background_color().into()),
                ..Default::default()
            },
        }
    }

    pub fn dialog_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered | button::Status::Active => button::Style {
                text_color: background_dark_color(),
                background: Some(primary_color().into()),
                ..Default::default()
            },
            _ => button::Style {
                text_color: background_dark_color(),
                background: Some(secondary_color().into()),
                ..Default::default()
            },
        }
    }

    pub fn copy_code_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered | button::Status::Active => button::Style {
                text_color: background_dark_color(),
                background: Some(primary_color().into()),
                ..Default::default()
            },
            _ => button::Style {
                text_color: background_dark_color(),
                background: Some(secondary_color().into()),
                ..Default::default()
            },
        }
    }

    pub fn text_editor_field(_theme: &Theme, _status: text_editor::Status) -> text_editor::Style {
        text_editor::Style {
            background: background_light_color().into(),
            border: Border {
                ..Default::default()
            },
            placeholder: text_color_dark().into(),
            value: text_color().into(),
            selection: highlight_color().into(),
        }
    }

    pub fn text_editor_container(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(background_light_color().into()),
            border: Border {
                color: border_color(),
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

pub mod viewers {
    use crate::styles::styles;
    use iced::widget::{button, hover, markdown, right, text};
    use iced::{Element, Task, clipboard};

    #[derive(Clone)]
    pub enum Interaction {
        Copy(String),
    }

    impl Interaction {
        pub fn perform<Message>(self) -> Task<Message> {
            match self {
                Interaction::Copy(text) => clipboard::write(text),
            }
        }
    }

    pub struct MarkdownViewer {}

    impl<'a> markdown::Viewer<'a, Interaction> for MarkdownViewer {
        // TODO: Open in browser
        fn on_link_click(url: markdown::Uri) -> Interaction {
            Interaction::Copy(url)
        }

        fn code_block(
            &self,
            settings: markdown::Settings,
            _language: Option<&'a str>,
            code: &'a str,
            lines: &'a [markdown::Text],
        ) -> Element<'a, Interaction> {
            let code_block = markdown::code_block(settings, lines, Interaction::Copy);

            let copy = button(text("Copy").size(settings.code_size))
                .on_press(Interaction::Copy(code.to_string()))
                .style(styles::copy_code_button)
                .padding(settings.code_size / 2);

            hover(code_block, right(copy))
        }
    }
}
