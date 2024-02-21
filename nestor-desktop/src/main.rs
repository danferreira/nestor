use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;

use iced::widget::{container, image};
use iced::Subscription;
use iced::{executor, Application, Command, Element, Length, Settings, Theme};
use nestor::NES;

pub fn main() -> iced::Result {
    let (sender, receiver) = mpsc::channel::<Vec<u8>>();

    thread::spawn(move || {
        let mut nes = NES::new("roms/games/dk.nes".to_owned());

        loop {
            let frame = nes.emulate_frame();
            if let Some(frame) = frame {
                let mut local_buffer: Vec<u8> = vec![];

                for color in frame.data.chunks_exact(3) {
                    local_buffer.push(color[0]);
                    local_buffer.push(color[1]);
                    local_buffer.push(color[2]);
                    local_buffer.push(255);
                }

                sender.send(local_buffer).unwrap();
            }
        }
    });

    let mut settings = Settings::with_flags(EmulatorFlags { receiver });
    settings.antialiasing = true;

    Emulator::run(settings)
}

struct EmulatorFlags {
    receiver: mpsc::Receiver<Vec<u8>>,
}

struct Emulator {
    receiver: RefCell<Option<mpsc::Receiver<Vec<u8>>>>,
    frame_buffer: Vec<u8>,
}

#[derive(Debug, Clone)]
enum Message {
    NewFrame(Vec<u8>),
}

impl Application for Emulator {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = EmulatorFlags;
    type Theme = Theme;

    fn new(flags: EmulatorFlags) -> (Self, Command<Message>) {
        let frame_buffer = vec![0; 256 * 240];
        (
            Emulator {
                receiver: RefCell::new(Some(flags.receiver)),
                frame_buffer,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Emulator")
    }

    fn theme(&self) -> iced::Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::NewFrame(frame) => {
                self.frame_buffer = frame;
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::subscription::unfold(
            "new frame",
            self.receiver.take(),
            move |mut receiver| async move {
                let frame = receiver.as_mut().unwrap().recv().unwrap();
                (Message::NewFrame(frame), receiver)
            },
        )
    }

    fn view(&self) -> Element<Message> {
        let img_handle = image::Handle::from_pixels(256, 240, self.frame_buffer.to_vec());

        let image: Element<Message> = image(img_handle)
            .filter_method(image::FilterMethod::Nearest)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        container(image)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
