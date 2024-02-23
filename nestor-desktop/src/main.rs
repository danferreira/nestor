use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use iced::keyboard::{key, Key};
use iced::widget::{container, image};
use iced::{executor, Application, Command, Element, Length, Settings, Theme};
use iced::{keyboard, Subscription};
use nestor::{JoypadButton, NES};

pub fn main() -> iced::Result {
    let (sender, receiver) = mpsc::channel::<Vec<u8>>();
    let nes = Arc::new(Mutex::new(NES::new()));
    let nes_clone = nes.clone();

    thread::spawn(move || loop {
        let mut nes_clone = nes_clone.lock().unwrap();

        if nes_clone.is_running {
            let frame = nes_clone.emulate_frame();
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

    let mut settings = Settings::with_flags(EmulatorFlags { nes, receiver });
    settings.antialiasing = true;

    Emulator::run(settings)
}

struct EmulatorFlags {
    nes: Arc<Mutex<NES>>,
    receiver: mpsc::Receiver<Vec<u8>>,
}

struct Emulator {
    nes: Arc<Mutex<NES>>,
    receiver: RefCell<Option<mpsc::Receiver<Vec<u8>>>>,
    frame_buffer: Vec<u8>,
}

#[derive(Debug, Clone)]
enum Message {
    NewFrame(Vec<u8>),
    OpenRom,
    RomOpened(Option<PathBuf>),
    ButtonPressed(JoypadButton),
    ButtonReleased(JoypadButton),
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
                nes: flags.nes,
                receiver: RefCell::new(Some(flags.receiver)),
                frame_buffer,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("NEStor - NES Emulator")
    }

    fn theme(&self) -> iced::Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::OpenRom => {
                self.nes.lock().unwrap().pause_emulation();
                Command::perform(open_rom(), Message::RomOpened)
            }
            Message::RomOpened(result) => {
                if let Some(path) = result {
                    self.nes.lock().unwrap().load_rom(path);
                    self.nes.lock().unwrap().start_emulation();
                } else {
                    self.nes.lock().unwrap().continue_emulation();
                }

                Command::none()
            }
            Message::ButtonPressed(button) => {
                self.nes.lock().unwrap().button_pressed(button, true);
                Command::none()
            }
            Message::ButtonReleased(button) => {
                self.nes.lock().unwrap().button_pressed(button, false);
                Command::none()
            }
            Message::NewFrame(frame) => {
                self.frame_buffer = frame;
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        fn get_joypad_button(key: Key) -> Option<JoypadButton> {
            match key.as_ref() {
                Key::Named(key::Named::ArrowDown) => Some(JoypadButton::DOWN),
                Key::Named(key::Named::ArrowUp) => Some(JoypadButton::UP),
                Key::Named(key::Named::ArrowRight) => Some(JoypadButton::RIGHT),
                Key::Named(key::Named::ArrowLeft) => Some(JoypadButton::LEFT),
                Key::Named(key::Named::Space) => Some(JoypadButton::SELECT),
                Key::Named(key::Named::Enter) => Some(JoypadButton::START),
                keyboard::Key::Character("a") => Some(JoypadButton::BUTTON_A),
                keyboard::Key::Character("s") => Some(JoypadButton::BUTTON_B),
                _ => None,
            }
        }

        let key_press_handler = keyboard::on_key_press(|key, _modifiers| match key.as_ref() {
            key::Key::Character("o") => Some(Message::OpenRom),
            _ => match get_joypad_button(key) {
                Some(joypad_button) => Some(Message::ButtonPressed(joypad_button)),
                None => None,
            },
        });

        let key_release_handler =
            keyboard::on_key_release(|key, _modifiers| match get_joypad_button(key) {
                Some(joypad_button) => Some(Message::ButtonReleased(joypad_button)),
                None => None,
            });

        let frame_handler = iced::subscription::unfold(
            "new frame",
            self.receiver.take(),
            move |mut receiver| async move {
                let frame = receiver.as_mut().unwrap().recv().unwrap();
                (Message::NewFrame(frame), receiver)
            },
        );

        Subscription::batch([key_press_handler, key_release_handler, frame_handler])
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

async fn open_rom() -> Option<PathBuf> {
    let path = std::env::current_dir().unwrap();

    let res = rfd::AsyncFileDialog::new()
        .add_filter("nes", &["nes"])
        .set_directory(&path)
        .pick_file()
        .await;

    if let Some(file) = res {
        Some(file.path().to_owned())
    } else {
        None
    }
}
