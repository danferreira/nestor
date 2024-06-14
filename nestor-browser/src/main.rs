use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use yew::{
    platform::{spawn_local, time::sleep},
    prelude::*,
};

use gloo::{console::info, dialogs::alert, file::File};
use nestor::NES;

use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement, ImageData};

use wasm_bindgen::{Clamped, JsCast, JsValue};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 240;
const SCALE: f64 = 3.0;

pub enum Msg {
    LoadRom(Vec<u8>),
    Tick(Vec<u8>),
    FileUpload(File),
}

pub struct App {
    emulator: Arc<Mutex<NES>>,
    canvas: NodeRef,
    ctx: Option<CanvasRenderingContext2d>,

    file_reader: Option<gloo::file::callbacks::FileReader>,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            emulator: Arc::new(Mutex::new(NES::new())),
            canvas: NodeRef::default(),
            ctx: None,
            file_reader: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::LoadRom(rom) => {
                // let emulator = self.emulator.lock().unwrap();
                self.emulator.lock().unwrap().load_rom_bytes(rom);
                info!("Rom loaded");

                let tick_cb = ctx.link().callback(Msg::Tick);
                emulate_frame(self.emulator.clone(), tick_cb);
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
            Msg::Tick(frame) => {
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
                    .draw_image_with_html_canvas_element(
                        &canvas_ctx.canvas().unwrap(),
                        0_f64,
                        0_f64,
                    )
                    .unwrap();

                true
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
                <input
                    id="file-input"
                    type="file"
                    multiple=false
                    accept=".nes"
                    onchange={
                        ctx.link().batch_callback(move |event: Event| {
                            let input: HtmlInputElement = event.target_unchecked_into();
                            if let Some(file) = input.files().map(|list| list.get(0)).flatten() {
                                Some(Msg::FileUpload(file.into()))
                            } else {
                                None
                            }
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

fn emulate_frame(emulator: Arc<Mutex<NES>>, cb: Callback<Vec<u8>>) {
    // Read endlessly from the UnboundedReceiver and compute the fun score.
    let clone = emulator.clone();
    spawn_local(async move {
        loop {
            if let Some(frame) = clone.lock().unwrap().emulate_frame() {
                let mut local_buffer: Vec<u8> = vec![];

                for color in frame.data.chunks_exact(3) {
                    local_buffer.push(color[0]);
                    local_buffer.push(color[1]);
                    local_buffer.push(color[2]);
                    local_buffer.push(255);
                }

                cb.emit(local_buffer);
                sleep(Duration::from_millis(16)).await;
            }
        }
    });
}
