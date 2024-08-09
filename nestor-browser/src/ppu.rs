use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::Closure, Clamped, JsCast};
use web_sys::{BroadcastChannel, HtmlCanvasElement, ImageData, MessageEvent};
use yew::{
    function_component, html, use_effect_with, use_mut_ref, use_node_ref, use_state, Html,
    Properties,
};

const SCALE: f64 = 2.0;

#[derive(Properties, PartialEq, Clone)]
pub struct PPUProps {
    pub pattern_table_0: Vec<u8>,
    pub pattern_table_1: Vec<u8>,
    pub palettes: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct PPUData {
    pub pattern_table_0: Vec<u8>,
    pub pattern_table_1: Vec<u8>,
    pub palettes: Vec<u8>,
}

#[function_component(PPU)]
pub fn ppu() -> Html {
    let canvas_pt0_ref = use_node_ref();
    let canvas_pt1_ref = use_node_ref();
    let canvas_palettes_ref = use_node_ref();
    let ctx_pt0_ref = use_mut_ref(|| None);
    let ctx_pt1_ref = use_mut_ref(|| None);
    let ctx_palettes_ref = use_mut_ref(|| None);
    let pattern_table_0_buffer = use_state(Vec::new);
    let pattern_table_1_buffer = use_state(Vec::new);
    let palettes_buffer = use_state(Vec::new);

    let broadcast_channel = BroadcastChannel::new("ppu").unwrap();

    {
        let pattern_table_0_buffer = pattern_table_0_buffer.clone();
        let pattern_table_1_buffer = pattern_table_1_buffer.clone();
        let palettes_buffer = palettes_buffer.clone();
        use_effect_with((), move |_| {
            let channel = broadcast_channel.clone();
            let listener = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(message) = serde_wasm_bindgen::from_value::<PPUData>(e.data()) {
                    pattern_table_0_buffer.set(message.pattern_table_0);
                    pattern_table_1_buffer.set(message.pattern_table_1);
                    palettes_buffer.set(message.palettes);
                }
            }) as Box<dyn FnMut(_)>);

            channel.set_onmessage(Some(listener.as_ref().unchecked_ref()));
            listener.forget();
        });
    }

    {
        let canvas_pt0_ref = canvas_pt0_ref.clone();
        let ctx_pt0_ref = ctx_pt0_ref.clone();

        use_effect_with(canvas_pt0_ref, move |canvas_pt0_ref| {
            if ctx_pt0_ref.borrow().is_none() {
                let canvas = canvas_pt0_ref
                    .cast::<HtmlCanvasElement>()
                    .expect("canvas_pt1_ref not attached to canvas element");

                let ctx = canvas
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();

                ctx.scale(SCALE, SCALE).unwrap();
                ctx.set_image_smoothing_enabled(false);

                ctx.set_fill_style(&"#000000".into());

                *ctx_pt0_ref.borrow_mut() = Some(ctx);
            }
        });
    }

    {
        let canvas_pt1_ref = canvas_pt1_ref.clone();
        let ctx_pt1_ref = ctx_pt1_ref.clone();

        use_effect_with(canvas_pt1_ref, move |canvas_pt1_ref| {
            if ctx_pt1_ref.borrow().is_none() {
                let canvas = canvas_pt1_ref
                    .cast::<HtmlCanvasElement>()
                    .expect("canvas_pt1_ref not attached to canvas element");

                let ctx = canvas
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();

                ctx.scale(SCALE, SCALE).unwrap();
                ctx.set_image_smoothing_enabled(false);

                ctx.set_fill_style(&"#000000".into());

                *ctx_pt1_ref.borrow_mut() = Some(ctx);
            }
        });
    }

    {
        let canvas_palettes_ref = canvas_palettes_ref.clone();
        let ctx_palettes_ref = ctx_palettes_ref.clone();

        use_effect_with(canvas_palettes_ref, move |canvas_palettes_ref| {
            if ctx_palettes_ref.borrow().is_none() {
                let canvas = canvas_palettes_ref
                    .cast::<HtmlCanvasElement>()
                    .expect("canvas_palettes_ref not attached to canvas element");

                let ctx = canvas
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();

                ctx.scale(SCALE, SCALE).unwrap();
                ctx.set_image_smoothing_enabled(false);

                ctx.set_fill_style(&"#000000".into());

                *ctx_palettes_ref.borrow_mut() = Some(ctx);
            }
        });
    }

    use_effect_with(
        pattern_table_0_buffer.clone(),
        move |pattern_table_0_buffer| {
            if let Some(ctx) = ctx_pt0_ref.borrow().as_ref() {
                if !pattern_table_0_buffer.is_empty() {
                    let img_data = ImageData::new_with_u8_clamped_array(
                        Clamped(pattern_table_0_buffer.as_slice()),
                        128,
                    )
                    .unwrap();

                    ctx.put_image_data(&img_data, 0.0, 0.0).unwrap();

                    ctx.draw_image_with_html_canvas_element(&ctx.canvas().unwrap(), 0_f64, 0_f64)
                        .unwrap();
                }
            }
        },
    );
    use_effect_with(
        pattern_table_1_buffer.clone(),
        move |pattern_table_1_buffer| {
            if let Some(ctx) = ctx_pt1_ref.borrow().as_ref() {
                if !pattern_table_1_buffer.is_empty() {
                    let img_data = ImageData::new_with_u8_clamped_array(
                        Clamped(pattern_table_1_buffer.as_slice()),
                        128,
                    )
                    .unwrap();

                    ctx.put_image_data(&img_data, 0.0, 0.0).unwrap();

                    ctx.draw_image_with_html_canvas_element(&ctx.canvas().unwrap(), 0_f64, 0_f64)
                        .unwrap();
                }
            }
        },
    );

    use_effect_with(palettes_buffer.clone(), move |palettes| {
        if let Some(ctx) = ctx_palettes_ref.borrow().as_ref() {
            if !palettes.is_empty() {
                let img_data =
                    ImageData::new_with_u8_clamped_array(Clamped(palettes.as_slice()), 256)
                        .unwrap();

                ctx.put_image_data(&img_data, 0.0, 0.0).unwrap();

                ctx.draw_image_with_html_canvas_element(&ctx.canvas().unwrap(), 0_f64, 0_f64)
                    .unwrap();
            }
        }
    });

    html! {
        <div class="ppu-viewer">
            <div class="pattern-tables">
                <fieldset>
                    <legend>{"Pattern table 1"}</legend>
                    <canvas
                        class="canvas-container"
                        width="256"
                        height="256"
                        ref={canvas_pt0_ref}>
                    </canvas>
                    </fieldset>
                <fieldset>
                    <legend>{"Pattern table 2"}</legend>
                    <canvas
                        class="canvas-container"
                        width="256"
                        height="256"
                        ref={canvas_pt1_ref}>
                    </canvas>
                </fieldset>
            </div>
            <div>
                <fieldset>
                    <legend>{"Palettes"}</legend>
                    <canvas
                        class="canvas-container"
                        width="512"
                        height="16"
                        ref={canvas_palettes_ref}>
                    </canvas>
                </fieldset>
            </div>
        </div>
    }
}
