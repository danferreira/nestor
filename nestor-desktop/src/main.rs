use fps_counter::FPSCounter;
use iced::keyboard::{key, Key};
use iced::widget::{center, container, horizontal_space, text, Row};
use iced::widget::{image, Column};
use iced::{futures, window, Size};
use iced::{keyboard, Border};
use iced::{Element, Length, Subscription, Task, Theme};
use menu::{menu_bar, Menu};

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use nestor::{JoypadButton, PlayerJoypad, NES, ROM};

mod menu;

fn main() -> iced::Result {
    iced::daemon(App::title, App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        .antialiasing(true)
        .run_with(App::new)
}

#[derive(Debug, Clone)]
pub enum Message {
    NewFrame(Vec<u8>),
    OpenRom,
    RomOpened(Option<PathBuf>),
    ButtonPressed(PlayerJoypad, JoypadButton, bool),
    OpenWindow(View),
    WindowOpened(window::Id, View),
    WindowClosed(window::Id),
    Dummy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Main,
    PPU,
    Nametables,
}

trait Window {
    fn title(&self) -> String;
    fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
    fn view(&self) -> Element<Message>;
}

#[derive(Default)]
struct App {
    nes: Arc<Mutex<NES>>,
    windows: BTreeMap<window::Id, (View, Box<dyn Window>)>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            App {
                nes: Arc::new(Mutex::new(NES::new())),
                windows: BTreeMap::new(),
            },
            window::open(window::Settings::default())
                .map(|id| Message::WindowOpened(id, View::Main)),
        )
    }
    fn title(&self, window: window::Id) -> String {
        self.windows
            .get(&window)
            .map(|(_, w)| w.title())
            .unwrap_or_default()
    }

    fn theme(&self, _window: window::Id) -> iced::Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenWindow(view) => {
                if let Some((id, _)) = self.windows.iter().find(|(_, (v, _))| *v == view) {
                    return window::gain_focus(*id);
                }

                window::open(window::Settings {
                    size: Size::new(800.0, 400.0),
                    ..Default::default()
                })
                .map(move |id| Message::WindowOpened(id, view.clone()))
            }
            Message::WindowOpened(id, view) => {
                let window: Box<dyn Window> = match view {
                    View::Main => Box::new(Emulator::new(self.nes.clone())),
                    View::PPU => Box::new(PPUWindow::new(self.nes.clone())),
                    View::Nametables => Box::new(NametablesWindow::new(self.nes.clone())),
                };
                self.windows.insert(id, (view, window));

                Task::none()
            }
            Message::WindowClosed(id) => {
                if let Some((View::Main, _w)) = self.windows.get(&id) {
                    return iced::exit();
                }

                self.windows.remove(&id);

                Task::none()
            }
            m => {
                let tasks: Vec<Task<Message>> = self
                    .windows
                    .iter_mut()
                    .map(|(_id, (_, w))| w.update(m.clone()))
                    .collect();

                Task::batch(tasks)
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions: Vec<Subscription<Message>> = self
            .windows
            .values()
            .map(|(_, w)| w.subscription())
            .collect();

        let window_events = window::close_events().map(Message::WindowClosed);
        subscriptions.push(window_events);

        Subscription::batch(subscriptions)
    }

    fn view(&self, window_id: window::Id) -> Element<Message> {
        if let Some((_, window)) = self.windows.get(&window_id) {
            center(window.view()).into()
        } else {
            horizontal_space().into()
        }
    }
}

#[derive(Default)]
struct Emulator {
    nes: Arc<Mutex<NES>>,
    receiver: RefCell<Option<mpsc::Receiver<Vec<u8>>>>,
    frame_buffer: Vec<u8>,
    is_running: bool,
    fps_counter: FPSCounter,
    fps: usize,
}

impl Emulator {
    fn new(nes: Arc<Mutex<NES>>) -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();

        {
            let nes = nes.clone();

            thread::spawn(move || {
                let wait_time = Duration::from_millis(16);
                let mut start = Instant::now();

                loop {
                    let mut nes = nes.lock().unwrap();

                    if nes.is_running() {
                        let frame = nes.emulate_frame();

                        if let Some(frame) = frame {
                            tx.send(frame.to_rgba()).unwrap();
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

impl Window for Emulator {
    fn title(&self) -> String {
        "NEStor - NES Emulator".into()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenRom => {
                self.nes.lock().unwrap().pause_emulation();
                Task::perform(open_rom(), Message::RomOpened)
            }
            Message::RomOpened(result) => {
                if let Some(path) = result {
                    match ROM::from_path(path) {
                        Ok(rom) => {
                            self.nes.lock().unwrap().insert_cartridge(rom);
                            self.is_running = true;
                        }
                        Err(error) => panic!("Failed on loading the rom: {error}"),
                    }
                } else {
                    self.nes.lock().unwrap().continue_emulation();
                }

                Task::none()
            }
            Message::ButtonPressed(player, button, pressed) => {
                self.nes
                    .lock()
                    .unwrap()
                    .button_pressed(player, button, pressed);
                Task::none()
            }
            Message::NewFrame(frame) => {
                self.frame_buffer = frame;
                self.fps = self.fps_counter.tick();

                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
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

    fn view(&self) -> Element<Message> {
        let file_menu = Menu::new("File").item("Open", Message::OpenRom).build();
        let debugger_menu = Menu::new("Debugger")
            .item("PPU", Message::OpenWindow(View::PPU))
            .item("Nametables", Message::OpenWindow(View::Nametables))
            .build();

        let mb = menu_bar(vec![file_menu, debugger_menu]);

        let mut cols = Column::new().push(mb);

        if self.is_running {
            let img_handle = image::Handle::from_rgba(256, 240, self.frame_buffer.to_vec());

            let image: Element<Message> = image(img_handle)
                .filter_method(image::FilterMethod::Nearest)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

            cols = cols.push(image).push(text(format!("FPS: {}", self.fps)));
        }

        container(cols)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

struct PPUWindow {
    nes: Arc<Mutex<NES>>,
}

impl PPUWindow {
    fn new(nes: Arc<Mutex<NES>>) -> Self {
        Self { nes }
    }
}

impl Window for PPUWindow {
    fn title(&self) -> String {
        "NEStor - PPU".into()
    }

    fn view(&self) -> Element<Message> {
        let nes = self.nes.lock().unwrap();
        let mut pt0_buffer = Vec::new();
        let mut pt1_buffer = Vec::new();
        let mut palette_buffer = Vec::new();

        if nes.is_running() {
            let (pattern_table_0, pattern_table_1) = nes.ppu_viewer();
            let palette: nestor::frame::Frame = nes.palette_viewer();

            pt0_buffer = pattern_table_0.to_rgba();
            pt1_buffer = pattern_table_1.to_rgba();
            palette_buffer = palette.to_rgba();
        }

        let pt_0_img_handle = image::Handle::from_rgba(128, 128, pt0_buffer);
        let pt_1_img_handle = image::Handle::from_rgba(128, 128, pt1_buffer);
        let palette_img_handle = image::Handle::from_rgba(256, 8, palette_buffer);

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
}

struct NametablesWindow {
    nes: Arc<Mutex<NES>>,
}

impl NametablesWindow {
    fn new(nes: Arc<Mutex<NES>>) -> Self {
        Self { nes }
    }
}

impl Window for NametablesWindow {
    fn title(&self) -> String {
        "NEStor - Nametables".into()
    }

    fn view(&self) -> Element<Message> {
        let nes = self.nes.lock().unwrap();
        let mut buffer = Vec::new();

        if nes.is_running() {
            buffer = nes.nametable_viewer().to_rgba();
        }

        let nt_img_handle = image::Handle::from_rgba(512, 480, buffer);
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
