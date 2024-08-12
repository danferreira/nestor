use gloo::events::EventListener;
use nestor::JoypadButton;
use std::borrow::Cow;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{HtmlCanvasElement, ImageData};
use yew::KeyboardEvent;
use yew::{
    function_component, hook, html, use_effect_with, use_mut_ref, use_node_ref, Callback, Html,
    Properties,
};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 224;

#[derive(Properties, PartialEq, Clone)]
pub struct EmulatorProps {
    pub frame: Vec<u8>,
    pub fps: Option<usize>,
    pub key_pressed: Callback<JoypadButton>,
    pub key_released: Callback<JoypadButton>,
}

fn joypad_from_key(key: &str) -> Option<JoypadButton> {
    match key {
        "ArrowUp" => Some(JoypadButton::UP),
        "ArrowDown" => Some(JoypadButton::DOWN),
        "ArrowLeft" => Some(JoypadButton::LEFT),
        "ArrowRight" => Some(JoypadButton::RIGHT),
        "z" => Some(JoypadButton::BUTTON_A),
        "x" => Some(JoypadButton::BUTTON_B),
        "a" => Some(JoypadButton::START),
        "s" => Some(JoypadButton::SELECT),
        _ => None,
    }
}

#[hook]
pub fn use_joypad_button<E, F>(event_type: E, callback: F)
where
    E: Into<Cow<'static, str>>,
    F: Fn(JoypadButton) + 'static,
{
    #[derive(PartialEq, Clone)]
    struct EventDependents {
        event_type: Cow<'static, str>,
        callback: Callback<JoypadButton>,
    }

    let deps = EventDependents {
        event_type: event_type.into(),
        callback: Callback::from(callback),
    };

    use_effect_with(deps, |deps| {
        let EventDependents {
            event_type,
            callback,
        } = deps.clone();

        let document = gloo::utils::document();

        let listener = EventListener::new(&document, event_type, move |e| {
            let key_event = e.clone().dyn_into::<KeyboardEvent>().unwrap();
            if let Some(key) = joypad_from_key(key_event.key().as_str()) {
                callback.emit(key);
            }
        });

        move || {
            drop(listener);
        }
    });
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

    {
        let key_pressed = props.key_pressed.clone();

        use_joypad_button("keydown", move |key| {
            key_pressed.emit(key);
        });
    }

    {
        let key_released = props.key_released.clone();
        use_joypad_button("keyup", move |key| {
            key_released.emit(key);
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
