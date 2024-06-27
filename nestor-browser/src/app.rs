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
use nestor::{JoypadButton, NES};
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

#[function_component(App)]
pub fn app() -> Html {
    let millis = use_state(|| 0);
    let frame_buffer = use_state(|| vec![]);
    let pattern_table_0_buffer = use_state(|| vec![]);
    let pattern_table_1_buffer = use_state(|| vec![]);
    let palettes_buffer = use_state(|| vec![]);
    let nametables_buffer = use_state(|| vec![]);

    let rom_bytes = use_state(|| None);
    let import_reading = use_state(|| Option::<FileReader>::None);
    let fps = use_state(|| 0);

    let emulator = use_mut_ref(|| NES::new());
    let fps_counter = use_mut_ref(|| FPSCounter::new());

    {
        let emulator = emulator.clone();

        use_effect_with((), move |_| {
            // Attach a keydown event listener to the document.
            let document = gloo::utils::document();

            let listener = EventListener::new(&document, "keydown", move |event| {
                let key_event = event.clone().dyn_into::<KeyboardEvent>().unwrap();
                if let Some(key) = joypad_from_key(key_event.key().as_str()) {
                    emulator.borrow_mut().button_pressed(key, true)
                }
            });

            || drop(listener)
        });
    }

    {
        let emulator = emulator.clone();

        use_effect_with((), move |_| {
            // Attach a keydown event listener to the document.
            let document = gloo::utils::document();

            let listener = EventListener::new(&document, "keyup", move |event| {
                let key_event = event.clone().dyn_into::<KeyboardEvent>().unwrap();
                if let Some(key) = joypad_from_key(key_event.key().as_str()) {
                    emulator.borrow_mut().button_pressed(key, false)
                }
            });

            || drop(listener)
        });
    }

    // Handle file change event
    let on_file_change = {
        let rom_bytes = rom_bytes.clone();
        let import_reading = import_reading.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();

            if let Some(files) = input.files() {
                let file: Blob = files.get(0).unwrap().into();

                import_reading.set(Some(read_as_bytes(&file, {
                    let rom_bytes = rom_bytes.clone();
                    move |bytes| match bytes {
                        Ok(bytes) => {
                            rom_bytes.set(Some(bytes));
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
        let rom_bytes = rom_bytes.clone();
        let emulator = emulator.clone();
        let millis = millis.clone();

        use_effect_with(rom_bytes, move |rom_bytes| {
            if let Some(bytes) = &**rom_bytes {
                emulator.borrow_mut().load_rom_bytes(bytes.clone());
                millis.set(16); // Start the emulator loop
            }
            || ()
        });
    }

    {
        let frame_buffer = frame_buffer.clone();
        let pattern_table_0_buffer = pattern_table_0_buffer.clone();
        let pattern_table_1_buffer = pattern_table_1_buffer.clone();
        let palettes_buffer = palettes_buffer.clone();
        let nametables_buffer = nametables_buffer.clone();
        let fps = fps.clone();

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

                            let (pattern_table_0, pattern_table_1) = emu_mut.ppu_viewer();
                            pattern_table_0_buffer.set(pattern_table_0.to_rgba());
                            pattern_table_1_buffer.set(pattern_table_1.to_rgba());

                            let palettes = emu_mut.palette_viewer().to_rgba();
                            palettes_buffer.set(palettes);

                            let nametables = emu_mut.nametable_viewer().to_rgba();
                            nametables_buffer.set(nametables);

                            fps.set(fps_counter.borrow_mut().tick());

                            break;
                        }
                    }
                }
            },
            *millis,
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
