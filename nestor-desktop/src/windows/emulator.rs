use iced::keyboard::{self, key, Key};
use iced::widget::{container, row, text, Stack};
use iced::widget::{image, Column};
use iced::{futures, Alignment, Pixels, Size};
use iced::{Element, Length, Subscription, Task};

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, RwLock};
use std::{
    thread,
    time::{Duration, Instant},
};

use fps_counter::FPSCounter;

use nestor::{JoypadButton, PlayerJoypad, NES, ROM};

use crate::menu::{menu_bar, Menu};

const NES_WIDTH: u32 = 256;
const NES_HEIGHT: u32 = 240;

#[derive(Debug, Clone)]
pub enum Message {
    NewFrame(Vec<u8>),
    OpenRom,
    RomOpened(Option<PathBuf>),
    ButtonPressed(PlayerJoypad, JoypadButton, bool),
    OpenPPU,
    OpenNametables,
    Dummy,
}

pub enum Action {
    Run(Task<Message>),
    OpenPPUWindow,
    OpenNametablesWindow,
}

pub struct Emulator {
    nes: Arc<RwLock<NES>>,
    receiver: RefCell<Option<mpsc::Receiver<Vec<u8>>>>,
    frame_buffer: Vec<u8>,
    is_running: bool,
    fps_counter: FPSCounter,
    fps: usize,
}

impl Emulator {
    pub fn new(nes: Arc<RwLock<NES>>) -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();

        {
            let nes = nes.clone();

            thread::spawn(move || {
                let wait_time = Duration::from_millis(16);
                let mut start = Instant::now();

                loop {
                    let mut nes = nes.write().unwrap();

                    if nes.is_running() {
                        let frame = nes.emulate_frame();

                        if let Some(frame) = frame {
                            let _ = tx.send(frame.to_rgba());
                            let runtime = start.elapsed();

                            if let Some(remaining) = wait_time.checked_sub(runtime) {
                                thread::sleep(remaining);
                            }

                            start = Instant::now()
                        }
                    }
                }
            });
        }

        Emulator {
            nes,
            receiver: RefCell::new(Some(rx)),
            frame_buffer: Vec::new(),
            is_running: false,
            fps_counter: FPSCounter::new(),
            fps: 0,
        }
    }
}

impl Emulator {
    fn title(&self) -> String {
        "NEStor - NES Emulator".into()
    }

    pub fn update(&mut self, message: Message) -> Option<Action> {
        match message {
            Message::OpenRom => {
                self.nes.write().unwrap().pause_emulation();
                Some(Action::Run(Task::perform(open_rom(), Message::RomOpened)))
            }
            Message::RomOpened(result) => {
                if let Some(path) = result {
                    match ROM::from_path(path) {
                        Ok(rom) => {
                            self.nes.write().unwrap().insert_cartridge(rom);
                            self.is_running = true;
                        }
                        Err(error) => panic!("Failed on loading the rom: {error}"),
                    }
                } else {
                    self.nes.write().unwrap().continue_emulation();
                }

                None
            }
            Message::ButtonPressed(player, button, pressed) => {
                self.nes
                    .write()
                    .unwrap()
                    .button_pressed(player, button, pressed);
                None
            }
            Message::NewFrame(frame) => {
                self.frame_buffer = frame;
                self.fps = self.fps_counter.tick();

                None
            }
            Message::OpenPPU => Some(Action::OpenPPUWindow),
            Message::OpenNametables => Some(Action::OpenNametablesWindow),
            Message::Dummy => None,
        }
    }

    pub fn settings(&self) -> iced::window::Settings {
        iced::window::Settings {
            size: Size::new((NES_WIDTH * 3) as f32, (NES_HEIGHT * 3) as f32),
            resizable: false,
            ..Default::default()
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        fn get_joypad_button(key: &Key) -> Option<(PlayerJoypad, JoypadButton)> {
            let player_one = match key.as_ref() {
                keyboard::Key::Character("w") => Some(JoypadButton::UP),
                keyboard::Key::Character("s") => Some(JoypadButton::DOWN),
                keyboard::Key::Character("a") => Some(JoypadButton::LEFT),
                keyboard::Key::Character("d") => Some(JoypadButton::RIGHT),
                keyboard::Key::Character("q") => Some(JoypadButton::SELECT),
                keyboard::Key::Character("e") => Some(JoypadButton::START),
                keyboard::Key::Character("f") => Some(JoypadButton::BUTTON_A),
                keyboard::Key::Character("g") => Some(JoypadButton::BUTTON_B),
                _ => None,
            };

            if let Some(button) = player_one {
                return Some((PlayerJoypad::One, button));
            }

            let player_two = match key.as_ref() {
                Key::Named(key::Named::ArrowUp) => Some(JoypadButton::UP),
                Key::Named(key::Named::ArrowDown) => Some(JoypadButton::DOWN),
                Key::Named(key::Named::ArrowLeft) => Some(JoypadButton::LEFT),
                Key::Named(key::Named::ArrowRight) => Some(JoypadButton::RIGHT),
                Key::Named(key::Named::Space) => Some(JoypadButton::SELECT),
                Key::Named(key::Named::Enter) => Some(JoypadButton::START),
                Key::Character("k") => Some(JoypadButton::BUTTON_A),
                Key::Character("l") => Some(JoypadButton::BUTTON_B),
                _ => None,
            };

            if let Some(button) = player_two {
                return Some((PlayerJoypad::Two, button));
            }

            None
        }

        let key_press_handler = keyboard::on_key_press(|key, _modifiers| {
            get_joypad_button(&key)
                .map(|(player, button)| Message::ButtonPressed(player, button, true))
        });

        let key_release_handler = keyboard::on_key_release(|key, _modifiers| {
            get_joypad_button(&key)
                .map(|(player, button)| Message::ButtonPressed(player, button, false))
        });

        let frame_streaming =
            futures::stream::unfold(self.receiver.take(), move |mut receiver| async {
                let frame = receiver.as_mut().unwrap().recv().unwrap();
                Some((Message::NewFrame(frame), receiver))
            });

        let frame_handler = Subscription::run_with_id("frames", frame_streaming);

        Subscription::batch([key_press_handler, key_release_handler, frame_handler])
    }

    pub fn view(&self) -> Element<Message> {
        let file_menu = Menu::new("File").item("Open", Message::OpenRom).build();
        let debugger_menu = Menu::new("Debugger")
            .item("PPU", Message::OpenPPU)
            .item("Nametables", Message::OpenNametables)
            .build();

        let mb = menu_bar(vec![file_menu, debugger_menu]);

        let mut cols = Column::new().push(mb);

        if self.is_running {
            let img_handle =
                image::Handle::from_rgba(NES_WIDTH, NES_HEIGHT, self.frame_buffer.to_vec());

            let image: Element<Message> = image(img_handle)
                .filter_method(image::FilterMethod::Nearest)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Fill)
                .into();

            let fps_text = row![text(self.fps)
                .size(Pixels(42.0))
                .width(Length::Fill)
                .align_x(Alignment::End)]
            .padding(20);

            let stack = Stack::new().push(image).push(fps_text);

            cols = cols.push(stack);
        }

        container(cols)
            .width(Length::Fill)
            .height(Length::Fill)
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

    res.map(|file| file.path().to_path_buf())
}
