use fps_counter::FPSCounter;
use yew::prelude::*;

use gloo::{
    dialogs::alert, events::EventListener, file::File, timers::callback::Interval, utils::document,
};
use nestor::{JoypadButton, NES};

use web_sys::HtmlInputElement;

use wasm_bindgen::JsCast;

mod display;

use display::Display;

pub enum Msg {
    LoadRom(Vec<u8>),
    Tick,
    FileUpload(File),
    KeyDown(JoypadButton),
    KeyUp(JoypadButton),
}

pub struct App {
    emulator: NES,

    file_reader: Option<gloo::file::callbacks::FileReader>,
    fps_counter: FPSCounter,
    frame: Vec<u8>,

    _interval: Interval,
    fps: usize,
    _key_up_listen: EventListener,
    _key_down_listen: EventListener,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        // Update frame every 16ms.
        let interval = {
            let link = ctx.link().clone();
            Interval::new(16, move || {
                link.send_message(Msg::Tick);
            })
        };

        // Callbacks for key events.
        let on_key_down = {
            let link = ctx.link().clone();
            Callback::from(move |e: KeyboardEvent| {
                match e.key().as_str() {
                    "ArrowUp" => link.send_message(Msg::KeyDown(JoypadButton::UP)),
                    "ArrowDown" => link.send_message(Msg::KeyDown(JoypadButton::DOWN)),
                    "ArrowLeft" => link.send_message(Msg::KeyDown(JoypadButton::LEFT)),
                    "ArrowRight" => link.send_message(Msg::KeyDown(JoypadButton::RIGHT)),
                    "z" => link.send_message(Msg::KeyDown(JoypadButton::BUTTON_A)),
                    "x" => link.send_message(Msg::KeyDown(JoypadButton::BUTTON_B)),
                    "Enter" => link.send_message(Msg::KeyDown(JoypadButton::START)),
                    "Shift" => link.send_message(Msg::KeyDown(JoypadButton::SELECT)),
                    _ => (),
                };
            })
        };

        let on_key_up = {
            let link = ctx.link().clone();
            Callback::from(move |e: KeyboardEvent| {
                match e.key().as_str() {
                    "ArrowUp" => link.send_message(Msg::KeyUp(JoypadButton::UP)),
                    "ArrowDown" => link.send_message(Msg::KeyUp(JoypadButton::DOWN)),
                    "ArrowLeft" => link.send_message(Msg::KeyUp(JoypadButton::LEFT)),
                    "ArrowRight" => link.send_message(Msg::KeyUp(JoypadButton::RIGHT)),
                    "z" => link.send_message(Msg::KeyUp(JoypadButton::BUTTON_A)),
                    "x" => link.send_message(Msg::KeyUp(JoypadButton::BUTTON_B)),
                    "Enter" => link.send_message(Msg::KeyUp(JoypadButton::START)),
                    "Shift" => link.send_message(Msg::KeyUp(JoypadButton::SELECT)),
                    _ => (),
                };
            })
        };

        // Attach key listeners to document.
        let doc = document();
        let key_down = EventListener::new(&doc, "keydown", move |event| {
            let key_event = event.clone().dyn_into::<KeyboardEvent>().unwrap();
            if !key_event.repeat() {
                on_key_down.emit(key_event);
            }
        });
        let key_up = EventListener::new(&doc, "keyup", move |event| {
            let key_event = event.clone().dyn_into::<KeyboardEvent>().unwrap();
            if !key_event.repeat() {
                on_key_up.emit(key_event);
            }
        });

        Self {
            emulator: NES::new(),
            file_reader: None,
            _interval: interval,
            fps_counter: FPSCounter::default(),
            fps: 0,
            _key_up_listen: key_up,
            _key_down_listen: key_down,
            frame: vec![],
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::LoadRom(rom) => {
                self.emulator.load_rom_bytes(rom);

                true
            }
            Msg::FileUpload(file) => {
                let link = ctx.link().clone();
                self.file_reader = Some(gloo::file::callbacks::read_as_bytes(
                    &file,
                    move |bytes| match bytes {
                        Ok(bytes) => {
                            link.send_message(Msg::LoadRom(bytes));
                        }

                        Err(e) => alert(&format!("Failed to read bytes: {}", e)),
                    },
                ));

                true
            }
            Msg::Tick => {
                if self.emulator.is_running() {
                    loop {
                        if let Some(frame) = self.emulator.emulate_frame() {
                            let mut local_buffer: Vec<u8> = vec![];

                            for color in frame.data.chunks_exact(3) {
                                local_buffer.push(color[0]);
                                local_buffer.push(color[1]);
                                local_buffer.push(color[2]);
                                local_buffer.push(255);
                            }

                            self.frame = local_buffer;
                            self.fps = self.fps_counter.tick();

                            break;
                        }
                    }
                }

                true
            }
            Msg::KeyDown(key) => {
                self.emulator.button_pressed(key, true);
                false
            }

            Msg::KeyUp(key) => {
                self.emulator.button_pressed(key, false);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <Display frame={self.frame.clone()} fps={self.fps.clone()}/>
                <input
                    id="file-input"
                    type="file"
                    multiple=false
                    accept=".nes"
                    onchange={
                        ctx.link().batch_callback(move |event: Event| {
                            let input: HtmlInputElement = event.target_unchecked_into();
                            input.files().and_then(|list| list.get(0)).map(|file| Msg::FileUpload(file.into()))
                        })
                    }
                />

            </div>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
