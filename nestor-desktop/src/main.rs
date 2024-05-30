use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::{thread, time};

use iced::keyboard::{key, Key};
use iced::multi_window::Application;
use iced::widget::{container, image, text, Column, Row};
use iced::{
    event, executor, multi_window, window, Border, Command, Element, Length, Settings, Size, Theme,
};
use iced::{keyboard, Subscription};
use nestor::{JoypadButton, NES};

pub fn main() -> iced::Result {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let nes = Arc::new(Mutex::new(NES::new()));
    let nes_clone = nes.clone();
    let mut frames = 0.0;
    let mut now = time::Instant::now();

    thread::spawn(move || loop {
        let mut nes_clone = nes_clone.lock().unwrap();

        if nes_clone.is_running() {
            let frame = nes_clone.emulate_frame();

            if let Some(frame) = frame {
                frames += 1.0;
                let elapsed = now.elapsed();

                if elapsed.as_secs_f64() >= 1.0 {
                    println!("FPS: {}", frames);
                    frames = 0.0;
                    now = time::Instant::now();
                }

                let mut local_buffer: Vec<u8> = vec![];

                for color in frame.data.chunks_exact(3) {
                    local_buffer.push(color[0]);
                    local_buffer.push(color[1]);
                    local_buffer.push(color[2]);
                    local_buffer.push(255);
                }

                // thread::sleep(time::Duration::from_millis(10));
                tx.send(local_buffer).unwrap();
            }
        }
    });

    let mut settings = Settings::with_flags(EmulatorFlags { nes, receiver: rx });
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
    is_running: bool,
    windows: HashMap<window::Id, Window>,
}

#[derive(Debug, Clone)]
enum Message {
    NewFrame(Vec<u8>),
    OpenRom,
    RomOpened(Option<String>),
    ButtonPressed(JoypadButton),
    ButtonReleased(JoypadButton),
    OpenWindow(Window),
    CloseWindow(window::Id),
    WindowClosed(window::Id),
}

#[derive(Debug, Clone)]
enum Window {
    PPUView,
    NametablesView,
}

impl multi_window::Application for Emulator {
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
                windows: HashMap::new(),
                is_running: false,
            },
            Command::none(),
        )
    }

    fn title(&self, window_id: window::Id) -> String {
        if window_id == window::Id::MAIN {
            String::from("NEStor - NES Emulator")
        } else {
            let window = self.windows.get(&window_id).unwrap();

            match window {
                Window::PPUView => String::from("NEStor - PPU Viewer"),
                Window::NametablesView => String::from("NEStor - Nametables Viewer"),
            }
        }
    }

    fn theme(&self, _window: window::Id) -> iced::Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::OpenWindow(debug_window) => {
                let (id, spawn_window) = window::spawn(window::Settings {
                    size: Size::new(800.0, 400.0),
                    ..Default::default()
                });
                self.windows.insert(id, debug_window);

                spawn_window
            }
            Message::CloseWindow(id) => window::close(id),
            Message::WindowClosed(id) => {
                self.windows.remove(&id);
                Command::none()
            }
            Message::OpenRom => {
                self.nes.lock().unwrap().pause_emulation();
                Command::perform(open_rom(), Message::RomOpened)
            }
            Message::RomOpened(result) => {
                if let Some(path) = result {
                    self.nes.lock().unwrap().load_rom(path);
                    self.nes.lock().unwrap().start_emulation();
                    self.is_running = true;
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
            key::Key::Character("p") => Some(Message::OpenWindow(Window::PPUView)),
            key::Key::Character("n") => Some(Message::OpenWindow(Window::NametablesView)),
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

        let event_listener = event::listen_with(|event, _| {
            if let iced::Event::Window(id, window_event) = event {
                match window_event {
                    window::Event::CloseRequested => Some(Message::CloseWindow(id)),
                    window::Event::Closed => Some(Message::WindowClosed(id)),
                    _ => None,
                }
            } else {
                None
            }
        });

        let frame_handler = iced::subscription::unfold(
            "new frame",
            self.receiver.take(),
            move |mut receiver| async move {
                let frame = receiver.as_mut().unwrap().recv().unwrap();
                (Message::NewFrame(frame), receiver)
            },
        );

        Subscription::batch([
            key_press_handler,
            key_release_handler,
            frame_handler,
            event_listener,
        ])
    }

    fn view(&self, window_id: window::Id) -> Element<Message> {
        if window_id == window::Id::MAIN {
            if self.is_running {
                self.render_emulator_view()
            } else {
                self.render_main_view()
            }
        } else {
            let window = self.windows.get(&window_id).unwrap();

            match window {
                Window::PPUView => self.render_ppu_view(),
                Window::NametablesView => self.render_nametables_view(),
            }
        }
    }
}

impl Emulator {
    fn render_main_view(&self) -> Element<Message> {
        let info = Column::new()
            .push(text("Press \"o\" to load a new rom"))
            .push("Press \"p\" to open PPU Viewer")
            .push("Press \"n\" to open Nametable Viewer");

        container(info)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn render_emulator_view(&self) -> Element<Message> {
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

    fn render_ppu_view(&self) -> Element<Message> {
        let mut pt_0_buffer: Vec<u8> = vec![0; 128 * 128 * 3];
        let mut pt_1_buffer: Vec<u8> = vec![0; 128 * 128 * 3];
        let mut palette_buffer: Vec<u8> = vec![0; 256 * 8 * 3];

        if self.is_running {
            let mut nes = self.nes.lock().unwrap();
            let (pattern_table_0, pattern_table_1) = nes.ppu_viewer();
            let palette = nes.palette_viewer();

            pt_0_buffer = self.convert_to_rgba(pattern_table_0.data);
            pt_1_buffer = self.convert_to_rgba(pattern_table_1.data);
            palette_buffer = self.convert_to_rgba(palette.data);
        } else {
            pt_0_buffer = self.convert_to_rgba(pt_0_buffer);
            pt_1_buffer = self.convert_to_rgba(pt_1_buffer);
            palette_buffer = self.convert_to_rgba(palette_buffer);
        }

        let pt_0_img_handle = image::Handle::from_pixels(128, 128, pt_0_buffer);
        let pt_1_img_handle = image::Handle::from_pixels(128, 128, pt_1_buffer);
        let palette_img_handle = image::Handle::from_pixels(256, 8, palette_buffer);

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
            .width(300)
            .height(300)
            .padding(20)
            .center_x()
            .center_y()
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();

                container::Appearance {
                    border: Border {
                        width: 2.0,
                        color: palette.primary.base.color,
                        ..Border::default()
                    },
                    ..Default::default()
                }
            });

        let pt_1_container = container(pt_1_image_ppu)
            .width(300)
            .height(300)
            .padding(20)
            .center_x()
            .center_y()
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();

                container::Appearance {
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
            .align_items(iced::Alignment::Center);

        container(columns)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn render_nametables_view(&self) -> Element<Message> {
        let mut nt_buffer: Vec<u8> = vec![0; 512 * 480 * 3];

        if self.is_running {
            let mut nes = self.nes.lock().unwrap();
            let nametable = nes.nametable_viewer();

            nt_buffer = self.convert_to_rgba(nametable.data);
        } else {
            nt_buffer = self.convert_to_rgba(nt_buffer);
        }

        let nt_img_handle = image::Handle::from_pixels(512, 480, nt_buffer);
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

    fn convert_to_rgba(&self, data: Vec<u8>) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        for color in data.chunks_exact(3) {
            buffer.push(color[0]);
            buffer.push(color[1]);
            buffer.push(color[2]);
            buffer.push(255);
        }

        buffer
    }
}

async fn open_rom() -> Option<String> {
    let path = std::env::current_dir().unwrap();

    let res = rfd::AsyncFileDialog::new()
        .add_filter("nes", &["nes"])
        .set_directory(&path)
        .pick_file()
        .await;

    if let Some(file) = res {
        Some(
            file.path()
                .to_path_buf()
                .into_os_string()
                .into_string()
                .unwrap(),
        )
    } else {
        None
    }
}
