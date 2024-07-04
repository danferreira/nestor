use iced::keyboard::{key, Key};
use iced::widget::{button, center, container, horizontal_space, text, Row};
use iced::widget::{image, Column};
use iced::window;
use iced::Size;
use iced::{keyboard, Border};
use iced::{Element, Length, Subscription, Task, Theme};
use iced_aw::menu::Item;
use iced_aw::menu::Menu;
use iced_aw::menu_bar;
use iced_aw::menu_items;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::{mpsc, Arc, Mutex};
use std::{thread, time};

use nestor::{JoypadButton, NES};

fn main() -> iced::Result {
    iced::daemon(App::title, App::update, App::view)
        .load(|| {
            window::open(window::Settings::default())
                .map(|id| Message::WindowOpened(id, View::MainView))
        })
        .theme(App::theme)
        .subscription(App::subscription)
        .antialiasing(true)
        .run_with(|| App {
            nes: Arc::new(Mutex::new(NES::new())),
            windows: BTreeMap::new(),
        })
}

#[derive(Debug, Clone)]
pub enum Message {
    NewFrame(Vec<u8>),
    OpenRom,
    RomOpened(Option<String>),
    ButtonPressed(JoypadButton),
    ButtonReleased(JoypadButton),
    OpenWindow(View),
    WindowOpened(window::Id, View),
    WindowClosed(window::Id),
}

#[derive(Debug, Clone)]
enum View {
    MainView,
    PPUView,
    NametablesView,
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
    windows: BTreeMap<window::Id, Box<dyn Window>>,
}

impl App {
    fn title(&self, window: window::Id) -> String {
        self.windows
            .get(&window)
            .map(|window| window.title())
            .unwrap_or_default()
    }

    fn theme(&self, _window: window::Id) -> iced::Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenWindow(window) => window::open(window::Settings {
                size: Size::new(800.0, 400.0),
                ..Default::default()
            })
            .map(move |id| Message::WindowOpened(id, window.clone())),
            Message::WindowOpened(id, window) => {
                match window {
                    View::MainView => {
                        self.windows
                            .insert(id, Box::new(Emulator::new(self.nes.clone())));
                    }
                    View::PPUView => {
                        self.windows
                            .insert(id, Box::new(PPUView::new(self.nes.clone())));
                    }
                    View::NametablesView => {
                        self.windows
                            .insert(id, Box::new(NametablesView::new(self.nes.clone())));
                    }
                }

                Task::none()
            }
            Message::WindowClosed(id) => {
                self.windows.remove(&id);

                if self.windows.is_empty() {
                    iced::exit()
                } else {
                    Task::none()
                }
            }
            m => {
                let tasks: Vec<Task<Message>> = self
                    .windows
                    .iter_mut()
                    .map(|(_id, w)| w.update(m.clone()))
                    .collect();

                Task::batch(tasks)
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions: Vec<Subscription<Message>> = self
            .windows
            .iter()
            .map(|(_id, w)| w.subscription())
            .collect();

        let window_events = window::close_events().map(Message::WindowClosed);
        subscriptions.push(window_events);

        Subscription::batch(subscriptions)
    }

    fn view(&self, window_id: window::Id) -> Element<Message> {
        if let Some(window) = self.windows.get(&window_id) {
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
}

impl Emulator {
    fn new(nes: Arc<Mutex<NES>>) -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let mut frames = 0.0;
        let mut now = time::Instant::now();

        let nes_clone = nes.clone();

        thread::spawn(move || loop {
            let mut nes = nes.lock().unwrap();

            if nes.is_running() {
                let frame = nes.emulate_frame();

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

                    tx.send(local_buffer).unwrap();
                }
            }
        });

        let frame_buffer = vec![0; 256 * 240];

        Emulator {
            nes: nes_clone,
            receiver: RefCell::new(Some(rx)),
            frame_buffer,
            is_running: false,
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
                    self.nes.lock().unwrap().load_rom(path);
                    self.nes.lock().unwrap().start_emulation();
                    self.is_running = true;
                } else {
                    self.nes.lock().unwrap().continue_emulation();
                }

                Task::none()
            }
            Message::ButtonPressed(button) => {
                self.nes.lock().unwrap().button_pressed(button, true);
                Task::none()
            }
            Message::ButtonReleased(button) => {
                self.nes.lock().unwrap().button_pressed(button, false);
                Task::none()
            }
            Message::NewFrame(frame) => {
                self.frame_buffer = frame;

                Task::none()
            }
            _ => Task::none(),
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

        let key_press_handler = keyboard::on_key_press(|key, _modifiers| {
            get_joypad_button(key).map(Message::ButtonPressed)
        });

        let key_release_handler = keyboard::on_key_release(|key, _modifiers| {
            get_joypad_button(key).map(Message::ButtonReleased)
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
        let file_items = menu_items!((menu_button("Open", Message::OpenRom)));
        let debug_items = menu_items!((menu_button("PPU", Message::OpenWindow(View::PPUView)))(
            menu_button("Nametables", Message::OpenWindow(View::NametablesView))
        ));

        let mb = menu_bar!((button("File"), Menu::new(file_items).max_width(180.0))(
            button("Debugger"),
            Menu::new(debug_items).max_width(180.0)
        ));

        let mut cols = Column::new().push(mb);

        if self.is_running {
            let img_handle = image::Handle::from_rgba(256, 240, self.frame_buffer.to_vec());

            let image: Element<Message> = image(img_handle)
                .filter_method(image::FilterMethod::Nearest)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

            cols = cols.push(image);
        }

        container(cols)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

struct PPUView {
    nes: Arc<Mutex<NES>>,
}

impl PPUView {
    fn new(nes: Arc<Mutex<NES>>) -> Self {
        Self { nes }
    }
}

impl Window for PPUView {
    fn title(&self) -> String {
        "NEStor - PPU View".into()
    }

    fn view(&self) -> Element<Message> {
        let mut nes = self.nes.lock().unwrap();
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
            .align_items(iced::Alignment::Center);

        container(columns)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

struct NametablesView {
    nes: Arc<Mutex<NES>>,
}

impl NametablesView {
    fn new(nes: Arc<Mutex<NES>>) -> Self {
        Self { nes }
    }
}

impl Window for NametablesView {
    fn title(&self) -> String {
        "NEStor - Nametable View".into()
    }

    fn view(&self) -> Element<Message> {
        let mut nes = self.nes.lock().unwrap();
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

fn menu_button(label: &str, msg: Message) -> button::Button<Message, iced::Theme, iced::Renderer> {
    button(text(label)).on_press(msg).width(Length::Fill)
}

async fn open_rom() -> Option<String> {
    let path = std::env::current_dir().unwrap();

    let res = rfd::AsyncFileDialog::new()
        .add_filter("nes", &["nes"])
        .set_directory(&path)
        .pick_file()
        .await;

    res.map(|file| {
        file.path()
            .to_path_buf()
            .into_os_string()
            .into_string()
            .unwrap()
    })
}
