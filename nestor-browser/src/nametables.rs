use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::Closure, Clamped, JsCast};
use web_sys::{BroadcastChannel, HtmlCanvasElement, ImageData, MessageEvent};
use yew::{function_component, html, use_effect_with, use_mut_ref, use_node_ref, use_state, Html};

#[derive(Serialize, Deserialize)]
pub struct NametablesData {
    pub nametables: Vec<u8>,
}

#[function_component(Nametables)]
pub fn nametables() -> Html {
    let canvas_ref = use_node_ref();
    let ctx_ref = use_mut_ref(|| None);
    let nametable_buffer = use_state(Vec::new);

    let broadcast_channel = BroadcastChannel::new("nametables").unwrap();
    {
        let nametable_buffer = nametable_buffer.clone();
        use_effect_with((), move |_| {
            let channel = broadcast_channel.clone();
            let listener = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(message) = serde_wasm_bindgen::from_value::<NametablesData>(e.data()) {
                    nametable_buffer.set(message.nametables);
                }
            }) as Box<dyn FnMut(_)>);

            channel.set_onmessage(Some(listener.as_ref().unchecked_ref()));
            listener.forget();
        });
    }

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

                *ctx_ref.borrow_mut() = Some(ctx);
            }
        });
    }

    {
        use_effect_with(nametable_buffer, move |nametable_buffer| {
            if let Some(ctx) = ctx_ref.borrow().as_ref() {
                if !nametable_buffer.is_empty() {
                    let img_data = ImageData::new_with_u8_clamped_array(
                        Clamped(nametable_buffer.as_slice()),
                        512,
                    )
                    .unwrap();

                    ctx.put_image_data(&img_data, 0.0, 0.0).unwrap();

                    ctx.draw_image_with_html_canvas_element(&ctx.canvas().unwrap(), 0_f64, 0_f64)
                        .unwrap();
                }
            }
        });
    }

    html! {
        <div class="nametables-viewer">
            <div class="nametables">
                <fieldset>
                    <legend>{"Nametables"}</legend>
                    <canvas
                        class="canvas-container"
                        width="512"
                        height="480"
                        ref={canvas_ref}>
                    </canvas>
                    </fieldset>
            </div>

        </div>
    }
}
