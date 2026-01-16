pub mod styles {
    use iced::widget::{button, container, text_editor};
    use iced::{Border, Color, Theme, color};

    pub fn sidebar(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(color!(0xDBDBDB).into()),
            background: Some(color!(0x2D2D2D).into()),
            border: Border {
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn messaging_area(_theme: &Theme) -> container::Style {
        container::Style {
            background: Some(color!(0x242424).into()),
            border: Border {
                ..Default::default()
            },
            ..Default::default()
        }
    }

    pub fn message(_theme: &Theme) -> container::Style {
        container::Style {
            text_color: Some(color!(0x000000).into()),
            background: Some(color!(0xDBDBDB).into()),
            border: Border {
                radius: 4.0.into(),
                color: color!(0xDBDBDB, 0.5),
                ..Default::default()
            },

            ..Default::default()
        }
    }

    pub fn new_chat_button(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered => button::Style {
                text_color: color!(0x000000).into(),
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
                text_color: color!(0x6F3AB2).into(),
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
                text_color: Color::BLACK,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                background: Some(color!(0xDBDBDB).into()),
                ..Default::default()
            },
            _ => button::Style {
                text_color: Color::WHITE,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                background: Some(color!(0x2D2D2D).into()),
                ..Default::default()
            },
        }
    }

    pub fn chat_selected(_theme: &Theme, status: button::Status) -> button::Style {
        match status {
            button::Status::Hovered | button::Status::Active => button::Style {
                text_color: Color::BLACK,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                background: Some(color!(0xDBDBDB).into()),
                ..Default::default()
            },
            _ => button::Style {
                text_color: Color::WHITE,
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                background: Some(color!(0x2D2D2D).into()),
                ..Default::default()
            },
        }
    }

    pub fn text_editor_field(_theme: &Theme, _status: text_editor::Status) -> text_editor::Style {
        text_editor::Style {
            background: color!(0xDBDBDB).into(),
            border: Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            placeholder: color!(0x5B5B5B).into(),
            value: Color::BLACK.into(),
            selection: Color::WHITE.into(),
        }
    }
}
