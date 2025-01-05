use iced::widget::{container, Row};
use iced::widget::{image, Column};
use iced::{futures, Border, Subscription};
use iced::{Element, Length, Theme};

use std::cell::RefCell;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

use nestor::NES;

#[derive(Debug)]
pub enum Message {
    NewFrame((Vec<u8>, Vec<u8>, Vec<u8>)),
}

pub enum Action {}

pub struct PPUWindow {
    receiver: RefCell<Option<mpsc::Receiver<(Vec<u8>, Vec<u8>, Vec<u8>)>>>,
    frame_buffer: (Vec<u8>, Vec<u8>, Vec<u8>),
}

impl PPUWindow {
    pub fn new(nes: Arc<RwLock<NES>>) -> Self {
        let (tx, rx) = mpsc::channel::<(Vec<u8>, Vec<u8>, Vec<u8>)>();

        {
            let nes = nes.clone();

            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(2));

                let nes = nes.read().unwrap();

                if nes.is_running() {
                    let (pattern_table_0, pattern_table_1) = nes.ppu_viewer();
                    let palette: nestor::frame::Frame = nes.palette_viewer();

                    let _ = tx.send((
                        pattern_table_0.to_rgba(),
                        pattern_table_1.to_rgba(),
                        palette.to_rgba(),
                    ));
                }
            });
        }

        Self {
            receiver: RefCell::new(Some(rx)),
            frame_buffer: (Vec::new(), Vec::new(), Vec::new()),
        }
    }
}

impl PPUWindow {
    pub fn title(&self) -> String {
        "NEStor - PPU".into()
    }

    pub fn settings(&self) -> iced::window::Settings {
        iced::window::Settings {
            size: iced::Size::new(768.0, 360.0),
            ..Default::default()
        }
    }

    pub fn view(&self) -> Element<Message> {
        let pt_0_img_handle = image::Handle::from_rgba(128, 128, self.frame_buffer.0.clone());
        let pt_1_img_handle = image::Handle::from_rgba(128, 128, self.frame_buffer.1.clone());
        let palette_img_handle = image::Handle::from_rgba(256, 8, self.frame_buffer.2.clone());

        let pt_0_image_ppu: Element<Message> = image(pt_0_img_handle)
            .filter_method(image::FilterMethod::Nearest)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let pt_1_image_ppu: Element<Message> = image(pt_1_img_handle)
            .filter_method(image::FilterMethod::Nearest)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let palette_image_ppu: Element<Message> = image(palette_img_handle)
            .filter_method(image::FilterMethod::Nearest)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        let pt_0_container = container(pt_0_image_ppu)
            .padding(20)
            .center_x(300)
            .center_y(300)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();

                container::Style {
                    border: Border {
                        width: 2.0,
                        color: palette.primary.base.color,
                        ..Border::default()
                    },
                    ..Default::default()
                }
            });

        let pt_1_container = container(pt_1_image_ppu)
            .padding(20)
            .center_x(300)
            .center_y(300)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();

                container::Style {
                    border: Border {
                        width: 2.0,
                        color: palette.primary.base.color,
                        ..Border::default()
                    },
                    ..Default::default()
                }
            });

        let pt_row = Row::new()
            .spacing(20)
            .push(pt_0_container)
            .push(pt_1_container);

        let columns = Column::new()
            .push(pt_row)
            .push(palette_image_ppu)
            .align_x(iced::Alignment::Center);

        container(columns)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn update(&mut self, message: Message) -> Option<Action> {
        match message {
            Message::NewFrame(frame) => {
                self.frame_buffer = frame;
                None
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let frame_streaming =
            futures::stream::unfold(self.receiver.take(), move |mut receiver| async {
                let frame = receiver.as_mut().unwrap().recv().unwrap();
                Some((Message::NewFrame(frame), receiver))
            });

        let frame_handler = Subscription::run_with_id("ppu", frame_streaming);

        Subscription::batch([frame_handler])
    }
}
