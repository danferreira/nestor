use iced::futures;
use iced::widget::container;
use iced::widget::image;
use iced::Subscription;
use iced::{Element, Length};

use std::cell::RefCell;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

use nestor::NES;

#[derive(Debug)]
pub enum Message {
    NewFrame(Vec<u8>),
}

pub enum Action {}

pub struct NametablesWindow {
    receiver: RefCell<Option<mpsc::Receiver<Vec<u8>>>>,
    frame_buffer: Vec<u8>,
}
impl NametablesWindow {
    pub fn new(nes: Arc<RwLock<NES>>) -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();

        {
            let nes = nes.clone();

            thread::spawn(move || loop {
                thread::sleep(Duration::from_secs(2));

                let nes = nes.read().unwrap();

                if nes.is_running() {
                    let buffer = nes.nametable_viewer().to_rgba();

                    let _ = tx.send(buffer);
                }
            });
        }

        Self {
            receiver: RefCell::new(Some(rx)),
            frame_buffer: Vec::new(),
        }
    }
}

impl NametablesWindow {
    pub fn title(&self) -> String {
        "NEStor - Nametables".into()
    }

    pub fn settings(&self) -> iced::window::Settings {
        iced::window::Settings {
            size: iced::Size::new(512 as f32, 512 as f32),
            ..Default::default()
        }
    }

    pub fn view(&self) -> Element<Message> {
        let nt_img_handle = image::Handle::from_rgba(512, 480, self.frame_buffer.clone());
        let nt_img: Element<Message> = image(nt_img_handle)
            .filter_method(image::FilterMethod::Nearest)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        container(nt_img)
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

        let frame_handler = Subscription::run_with_id("nametables", frame_streaming);

        Subscription::batch([frame_handler])
    }
}
