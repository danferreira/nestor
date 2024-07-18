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

    fn menu_button_item(
        label: String,
        message: Message,
    ) -> button::Button<'a, Message, iced::Theme, iced::Renderer> {
        button(text(label))
            .style(|_theme, status| match status {
                button::Status::Hovered => button::Style {
                    text_color: Color::WHITE,
                    background: Some(Color::from_rgb8(159, 28, 36).into()),
                    ..Default::default()
                },
                _ => button::Style {
                    text_color: Color::WHITE,
                    ..Default::default()
                },
            })
            .on_press(message)
    }

    pub fn item(mut self, label: &'a str, message: Message) -> Self {
        let item = Menu::menu_button_item(label.to_string(), message).width(Length::Fill);

        self.items.push(menu::Item::new(item));
        self
    }

    pub fn build(self) -> Item<'a, Message, Theme, Renderer> {
        let item = Menu::menu_button_item(self.label, Message::Dummy);

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
            bar_background: Color::from_rgb8(94, 94, 94).into(),
            bar_background_expand: 0.into(),
            menu_background: Color::from_rgb8(94, 94, 94).into(),
            menu_border: Border {
                radius: 0.0.into(),
                ..Default::default()
            },
            menu_background_expand: 0.into(),
            ..Default::default()
        })
        .width(Length::Fill)
}
