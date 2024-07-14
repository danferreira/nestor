use crate::display::Display;
use crate::nametables::Nametables;
use crate::ppu::PPU;

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
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_hooks::use_interval;

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

#[function_component(App)]
pub fn app() -> Html {
    let emulator = use_mut_ref(NES::new);
    let fps_counter = use_mut_ref(FPSCounter::new);

    let fps = use_state_eq(|| 0);
    let millis = use_state_eq(|| 0);
    let frame_buffer = use_state(Vec::new);
    let pattern_table_0_buffer = use_state(Vec::new);
    let pattern_table_1_buffer = use_state(Vec::new);
    let palettes_buffer = use_state(Vec::new);
    let nametables_buffer = use_state(Vec::new);
    let import_reading = use_state(|| Option::<FileReader>::None);

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

    // Handle file change event
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
                                Err(error) => info!(format!("Failed on loading the rom: {error}")),
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
                            let mut local_buffer: Vec<u8> = vec![];

                            for color in frame.data.chunks_exact(3) {
                                local_buffer.push(color[0]);
                                local_buffer.push(color[1]);
                                local_buffer.push(color[2]);
                                local_buffer.push(255);
                            }

                            frame_buffer.set(local_buffer);
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
        let pattern_table_0_buffer = pattern_table_0_buffer.clone();
        let pattern_table_1_buffer = pattern_table_1_buffer.clone();
        let palettes_buffer = palettes_buffer.clone();
        let nametables_buffer = nametables_buffer.clone();

        use_interval(
            move || {
                if emulator.borrow().is_running() {
                    let (pattern_table_0, pattern_table_1) = emulator.borrow_mut().ppu_viewer();
                    pattern_table_0_buffer.set(pattern_table_0.to_rgba());
                    pattern_table_1_buffer.set(pattern_table_1.to_rgba());

                    let palettes = emulator.borrow_mut().palette_viewer().to_rgba();
                    palettes_buffer.set(palettes);

                    let nametables = emulator.borrow_mut().nametable_viewer().to_rgba();
                    nametables_buffer.set(nametables);
                }
            },
            1000,
        );
    }

    html! {
        <div class="main-container">
            <div class="emulator">
                <Display frame={(*frame_buffer).clone()} fps={*fps}/>
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
            <div class="debugger">
                <PPU pattern_table_0={(*pattern_table_0_buffer).clone()}  pattern_table_1={(*pattern_table_1_buffer).clone()} palettes={(*palettes_buffer).clone()} />
                <Nametables nametables={(*nametables_buffer).clone()} />
            </div>
        </div>
    }
}
