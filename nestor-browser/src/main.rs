use fps_counter::FPSCounter;
use yew::prelude::*;

use gloo::{
    console::info, dialogs::alert, events::EventListener, file::File, timers::callback::Interval,
    utils::document,
};
use nestor::{JoypadButton, NES};

use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement, ImageData};

use wasm_bindgen::{Clamped, JsCast, JsValue};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 240;
const SCALE: f64 = 3.0;

pub enum Msg {
    LoadRom(Vec<u8>),
    Tick,
    FileUpload(File),
    KeyDown(JoypadButton),
    KeyUp(JoypadButton),
}

pub struct App {
    // emulator: Arc<Mutex<NES>>,
    emulator: NES,
    canvas: NodeRef,
    ctx: Option<CanvasRenderingContext2d>,

    file_reader: Option<gloo::file::callbacks::FileReader>,
    fps_counter: FPSCounter,

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
            canvas: NodeRef::default(),
            ctx: None,
            file_reader: None,
            _interval: interval,
            fps_counter: FPSCounter::default(),
            fps: 0,
            _key_up_listen: key_up,
            _key_down_listen: key_down,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::LoadRom(rom) => {
                self.emulator.load_rom_bytes(rom);
                info!("Rom loaded");

                true
            }
            Msg::FileUpload(file) => {
                info!("File Upload");
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

                            self.render_frame(local_buffer);

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
                <canvas
                    width={(WIDTH * SCALE as u32).to_string()}
                    height={(HEIGHT * SCALE as u32).to_string()}
                    ref={self.canvas.clone()}>
                </canvas>
                <div>{"FPS: "} {{self.fps}}</div>
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

impl App {
    fn render_frame(&mut self, frame: Vec<u8>) {
        let canvas = self.canvas.cast::<HtmlCanvasElement>().unwrap();
        let canvas_ctx = match &self.ctx {
            Some(canvas_ctx) => canvas_ctx,
            None => {
                let canvas_ctx = canvas.get_context("2d").unwrap().unwrap();
                let canvas_ctx: CanvasRenderingContext2d = canvas_ctx
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();
                canvas_ctx.scale(SCALE, SCALE).unwrap();
                canvas_ctx.set_fill_style(&JsValue::from("black"));
                canvas_ctx.set_image_smoothing_enabled(false);

                self.ctx = Some(canvas_ctx);
                self.ctx.as_ref().unwrap()
            }
        };

        let img_data =
            ImageData::new_with_u8_clamped_array(Clamped(frame.as_slice()), WIDTH).unwrap();

        canvas_ctx.put_image_data(&img_data, 0.0, 0.0).unwrap();

        canvas_ctx
            .draw_image_with_html_canvas_element(&canvas_ctx.canvas().unwrap(), 0_f64, 0_f64)
            .unwrap();

        self.fps = self.fps_counter.tick();
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
