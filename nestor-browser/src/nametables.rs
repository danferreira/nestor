use web_sys::{HtmlCanvasElement, ImageData};
use yew::{function_component, html, use_effect_with, use_mut_ref, use_node_ref, Html, Properties};

use wasm_bindgen::{prelude::*, Clamped};

#[derive(Properties, PartialEq, Clone)]
pub struct NametablesProps {
    pub nametables: Vec<u8>,
}

#[function_component(Nametables)]
pub fn nametables(props: &NametablesProps) -> Html {
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

                *ctx_ref.borrow_mut() = Some(ctx);
            }
        });
    }

    {
        use_effect_with(props.nametables.clone(), move |nametable_buffer| {
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
