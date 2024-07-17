use iced::{
    widget::{button, text},
    Border, Color, Length, Renderer, Theme,
};
use iced_aw::{
    menu::{self, Item, MenuBar},
    style::menu_bar,
};

use crate::Message;

pub struct Menu<'a> {
    label: String,
    items: Vec<Item<'a, Message, Theme, Renderer>>,
}

impl<'a> Menu<'a> {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            items: Vec::new(),
        }
    }

    pub fn item(mut self, label: &str, message: Message) -> Self {
        let item: button::Button<Message, iced::Theme, iced::Renderer> =
            button(text(label.to_string()))
                .style(|_theme, status| match status {
                    button::Status::Hovered => button::Style {
                        text_color: Color::WHITE,
                        background: Some(Color::from_rgba(0.9, 0.0, 0.0, 0.5).into()),
                        ..Default::default()
                    },
                    _ => button::Style {
                        text_color: Color::WHITE,
                        ..Default::default()
                    },
                })
                .on_press(message)
                .width(Length::Fill);

        let menu_item = menu::Item::new(item);
        self.items.push(menu_item);
        self
    }

    pub fn build(self) -> Item<'a, Message, Theme, Renderer> {
        let item = button(text(self.label))
            .style(|_theme, status| match status {
                button::Status::Hovered => button::Style {
                    text_color: Color::WHITE,
                    background: Some(Color::from_rgba(0.9, 0.0, 0.0, 0.5).into()),
                    ..Default::default()
                },
                _ => button::Style {
                    text_color: Color::WHITE,
                    ..Default::default()
                },
            })
            .on_press(Message::Dummy);

        menu::Item::with_menu(
            item,
            menu::Menu::new(self.items).max_width(180.0).spacing(0.0),
        )
    }
}

pub(crate) fn menu_bar(
    items: Vec<Item<Message, Theme, Renderer>>,
) -> MenuBar<Message, Theme, Renderer> {
    menu::MenuBar::new(items)
        .style(|_theme, _status| menu_bar::Style {
            bar_background: Color::BLACK.into(),
            bar_background_expand: 0.into(),
            menu_background: Color::BLACK.into(),
            menu_border: Border {
                radius: 0.0.into(),
                ..Default::default()
            },
            menu_background_expand: 0.into(),
            ..Default::default()
        })
        .width(Length::Fill)
}
