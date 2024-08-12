use wasm_bindgen::{Clamped, JsCast};
use web_sys::{HtmlCanvasElement, ImageData};
use yew::{function_component, html, use_effect_with, use_mut_ref, use_node_ref, Html, Properties};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 224;

#[derive(Properties, PartialEq, Clone)]
pub struct EmulatorProps {
    pub frame: Vec<u8>,
    pub fps: Option<usize>,
}

#[function_component(Emulator)]
pub fn emulator(props: &EmulatorProps) -> Html {
    let canvas_ref = use_node_ref();
    let ctx_ref = use_mut_ref(|| None);

    {
        let canvas_ref = canvas_ref.clone();
        let ctx_ref = ctx_ref.clone();

        use_effect_with(canvas_ref, move |canvas_ref| {
            if ctx_ref.borrow().is_none() {
                let canvas = canvas_ref
                    .cast::<HtmlCanvasElement>()
                    .expect("canvas_ref not attached to canvas element");

                let ctx = canvas
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();

                ctx.set_image_smoothing_enabled(false);

                ctx.set_fill_style(&"#000000".into());
                ctx.fill_rect(0_f64, 0_f64, WIDTH as f64, HEIGHT as f64);

                *ctx_ref.borrow_mut() = Some(ctx);
            }
        });
    }

    {
        let frame = props.frame.clone();

        use_effect_with(frame, move |frame| {
            if let Some(ctx) = ctx_ref.borrow().as_ref() {
                if !frame.is_empty() {
                    let img_data =
                        ImageData::new_with_u8_clamped_array(Clamped(frame.as_slice()), WIDTH)
                            .unwrap();

                    ctx.put_image_data(&img_data, 0.0, 0.0).unwrap();
                }
            }
        });
    }

    html! {
    <div>
        <canvas class="full-canvas-container" width="256" height="240" ref={canvas_ref}></canvas>
        if let Some(fps) = props.fps {
            <div class="fps-counter">{fps}</div>
        }
    </div>
    }
}
