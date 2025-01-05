use iced::widget::horizontal_space;
use iced::window;
use iced::{Element, Subscription, Task, Theme};
use windows::{emulator, nametables, ppu};

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use nestor::NES;

mod menu;
mod windows;

fn main() -> iced::Result {
    iced::daemon(App::title, App::update, App::view)
        .theme(App::theme)
        .subscription(App::subscription)
        .antialiasing(true)
        .run_with(App::new)
}

#[derive(Debug)]
pub enum Message {
    WindowClosed(window::Id),
    EmulatorMessage(window::Id, emulator::Message),
    PPUMessage(window::Id, ppu::Message),
    NametablesMessage(window::Id, nametables::Message),
    Dummy,
}

pub enum Window {
    Emulator(emulator::Emulator),
    PPU(ppu::PPUWindow),
    Nametables(nametables::NametablesWindow),
}

struct App {
    nes: Arc<RwLock<NES>>,
    windows: BTreeMap<window::Id, Window>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let nes = Arc::new(RwLock::new(NES::new()));
        let emulator = emulator::Emulator::new(nes.clone());
        let mut windows = BTreeMap::new();

        let (id, task) = window::open(emulator.settings());
        windows.insert(id, Window::Emulator(emulator));

        (App { nes, windows }, task.map(|_id| Message::Dummy))
    }

    fn title(&self, _window_id: window::Id) -> String {
        "Emulator".into()
    }

    fn theme(&self, _window: window::Id) -> iced::Theme {
        Theme::Dark
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowClosed(id) => {
                if let Some(Window::Emulator(_)) = self.windows.get(&id) {
                    return iced::exit();
                }

                self.windows.remove(&id);

                Task::none()
            }
            Message::EmulatorMessage(id, message) => {
                if let Some(Window::Emulator(emulator)) = self.windows.get_mut(&id) {
                    if let Some(action) = emulator.update(message) {
                        match action {
                            emulator::Action::Run(task) => {
                                return task.map(move |m| Message::EmulatorMessage(id, m))
                            }
                            emulator::Action::OpenPPUWindow => {
                                let window = ppu::PPUWindow::new(self.nes.clone());
                                let (id, task) = window::open(window.settings());

                                self.windows.insert(id, Window::PPU(window));

                                return task.map(|_id| Message::Dummy);
                            }
                            emulator::Action::OpenNametablesWindow => {
                                let window = nametables::NametablesWindow::new(self.nes.clone());
                                let (id, task) = window::open(window.settings());

                                self.windows.insert(id, Window::Nametables(window));
                                return task.map(|_id| Message::Dummy);
                            }
                        }
                    }
                }

                Task::none()
            }
            Message::PPUMessage(id, message) => {
                if let Some(Window::PPU(ppu)) = self.windows.get_mut(&id) {
                    if let Some(_action) = ppu.update(message) {}
                }
                Task::none()
            }
            Message::NametablesMessage(id, message) => {
                if let Some(Window::Nametables(nametables)) = self.windows.get_mut(&id) {
                    if let Some(_action) = nametables.update(message) {}
                }
                Task::none()
            }
            Message::Dummy => Task::none(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions: Vec<Subscription<Message>> = self
            .windows
            .iter()
            .map(|(id, w)| match w {
                Window::Emulator(window) => {
                    let id_cloned = id.clone();
                    window
                        .subscription()
                        .with(id_cloned)
                        .map(move |(id, m)| Message::EmulatorMessage(id, m))
                }
                Window::PPU(window) => {
                    let id_cloned = id.clone();
                    window
                        .subscription()
                        .with(id_cloned)
                        .map(move |(id, m)| Message::PPUMessage(id, m))
                }
                Window::Nametables(window) => {
                    let id_cloned = id.clone();
                    window
                        .subscription()
                        .with(id_cloned)
                        .map(move |(id, m)| Message::NametablesMessage(id, m))
                }
            })
            .collect();

        let window_events = window::close_events().map(Message::WindowClosed);
        subscriptions.push(window_events);

        Subscription::batch(subscriptions)
    }

    fn view(&self, window_id: window::Id) -> Element<Message> {
        if let Some(window) = self.windows.get(&window_id) {
            match window {
                Window::Emulator(window) => window
                    .view()
                    .map(move |m| Message::EmulatorMessage(window_id, m)),
                Window::PPU(window) => window
                    .view()
                    .map(move |m| Message::PPUMessage(window_id, m)),
                Window::Nametables(window) => window
                    .view()
                    .map(move |m| Message::NametablesMessage(window_id, m)),
            }
        } else {
            horizontal_space().into()
        }
    }
}
