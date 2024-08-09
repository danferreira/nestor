use crate::nametables::{Nametables, NametablesData};
use crate::ppu::PPU;
use crate::{display::Display, ppu::PPUData};

use fps_counter::FPSCounter;
use gloo::{
    console::info,
    events::EventListener,
    file::{
        callbacks::{read_as_bytes, FileReader},
        Blob,
    },
};
use nestor::{JoypadButton, PlayerJoypad, NES, ROM};
use std::borrow::Cow;
use wasm_bindgen::JsCast;
use web_sys::{js_sys::wasm_bindgen, HtmlInputElement};
use yew::prelude::*;
use yew_hooks::use_interval;
use yew_router::prelude::*;

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

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Emulator,
    #[at("/ppu")]
    PPU,
    #[at("/nametables")]
    Nametables,
}

#[cfg(feature = "tauri")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    pub async fn listen(event_name: &str, cb: &Closure<dyn Fn(JsValue)>) -> JsValue;
}

#[cfg(feature = "tauri")]
#[derive(Clone, Debug, serde::Deserialize)]
pub struct ListenPayload<T> {
    pub payload: T,
}

#[function_component(Emulator)]
fn emulator() -> Html {
    let emulator = use_mut_ref(NES::new);
    let fps_counter = use_mut_ref(FPSCounter::new);

    let fps = use_state_eq(|| 0);
    let millis = use_state_eq(|| 0);
    let frame_buffer = use_state(Vec::new);
    let import_reading = use_state(|| Option::<FileReader>::None);
    let ppu_broadcast_channel = use_state(|| web_sys::BroadcastChannel::new("ppu").unwrap());
    let nametables_broadcast_channel =
        use_state(|| web_sys::BroadcastChannel::new("nametables").unwrap());

    {
        let emulator = emulator.clone();

        use_joypad_button("keydown", move |key| {
            emulator
                .borrow_mut()
                .button_pressed(PlayerJoypad::One, key, true)
        })
    }

    {
        let emulator = emulator.clone();

        use_joypad_button("keyup", move |key| {
            emulator
                .borrow_mut()
                .button_pressed(PlayerJoypad::One, key, false)
        });
    }

    {
        let frame_buffer = frame_buffer.clone();
        let fps = fps.clone();
        let emulator = emulator.clone();

        use_interval(
            move || {
                if emulator.borrow().is_running() {
                    loop {
                        let mut emu_mut = emulator.borrow_mut();
                        if let Some(frame) = emu_mut.emulate_frame() {
                            frame_buffer.set(frame.to_rgba());
                            fps.set(fps_counter.borrow_mut().tick());

                            break;
                        }
                    }
                }
            },
            *millis,
        );
    }

    {
        let emulator = emulator.clone();

        use_interval(
            move || {
                if emulator.borrow().is_running() {
                    let (pattern_table_0, pattern_table_1) = emulator.borrow_mut().ppu_viewer();
                    let palettes = emulator.borrow_mut().palette_viewer();

                    let ppu_data = PPUData {
                        pattern_table_0: pattern_table_0.to_rgba(),
                        pattern_table_1: pattern_table_1.to_rgba(),
                        palettes: palettes.to_rgba(),
                    };

                    let serialized_value = serde_wasm_bindgen::to_value(&ppu_data).unwrap();

                    ppu_broadcast_channel
                        .post_message(&serialized_value)
                        .unwrap();

                    let nametables = emulator.borrow_mut().nametable_viewer().to_rgba();
                    let nametable_data = NametablesData { nametables };

                    let serialized_value = serde_wasm_bindgen::to_value(&nametable_data).unwrap();

                    nametables_broadcast_channel
                        .post_message(&serialized_value)
                        .unwrap();
                }
            },
            1000,
        );
    }

    #[cfg(feature = "tauri")]
    {
        let emulator = emulator.clone();
        let millis = millis.clone();

        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let cb = wasm_bindgen::prelude::Closure::wrap(Box::new(
                    move |event: wasm_bindgen::JsValue| {
                        let payload =
                            serde_wasm_bindgen::from_value::<ListenPayload<Vec<u8>>>(event)
                                .unwrap();
                        let rom = ROM::from_bytes(&payload.payload).unwrap();
                        emulator.borrow_mut().insert_cartridge(rom);
                        millis.set(16);
                    },
                )
                    as Box<dyn Fn(wasm_bindgen::JsValue)>);

                let _ = listen("load_rom", &cb).await;
                cb.forget();
            });

            || {}
        });
    }

    if cfg!(feature = "tauri") {
        html! {
        <div>
            <Display frame={(*frame_buffer).clone()} fps={*fps}/>
        </div>
        }
    } else {
        // // Handle file change event
        let on_file_change = {
            let import_reading = import_reading.clone();
            let emulator = emulator.clone();
            let millis = millis.clone();

            Callback::from(move |e: Event| {
                let input: HtmlInputElement = e.target_unchecked_into();

                if let Some(files) = input.files() {
                    let file: Blob = files.get(0).unwrap().into();

                    import_reading.set(Some(read_as_bytes(&file, {
                        let emulator = emulator.clone();
                        let millis = millis.clone();

                        move |bytes| match bytes {
                            Ok(bytes) => {
                                match ROM::from_bytes(&bytes) {
                                    Ok(rom) => {
                                        emulator.borrow_mut().insert_cartridge(rom);
                                        millis.set(16); // Start the emulator loop
                                    }
                                    Err(error) => {
                                        info!(format!("Failed on loading the rom: {error}"))
                                    }
                                }
                            }
                            Err(_e) => {
                                info!("Error when reading the file");
                            }
                        }
                    })));
                }
            })
        };

        html! {
            <div class="container">
                <div class="canvas-container">
                    <Display frame={(*frame_buffer).clone()} fps={*fps}/>
                </div>
                    <div class="emulator-controls">
                        <fieldset>
                        <legend>{"Actions"}</legend>
                        <input
                            id="file-input"
                            type="file"
                            multiple=false
                            accept=".nes"
                            onchange={on_file_change}
                        />
                        </fieldset>
                    </div>
            </div>
        }
    }
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Emulator => html! { <Emulator />},
        Route::PPU => html! {
            <PPU />
        },
        Route::Nametables => html! { <Nametables />},
    }
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} /> // <- must be child of <BrowserRouter>
        </BrowserRouter>
    }
}
