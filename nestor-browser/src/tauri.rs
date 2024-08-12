use crate::emulator::Emulator;
use crate::nametables::Nametables;
use crate::ppu::PPU;

use fps_counter::FPSCounter;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::js_sys::Uint8Array;
use yew::{function_component, html, use_mut_ref, use_state_eq, Html};
use yew_hooks::{use_async, use_interval};

#[derive(Serialize, Deserialize, Clone)]
pub struct NametablesData {
    pub nametables: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PPUData {
    pub pattern_table_0: Vec<u8>,
    pub pattern_table_1: Vec<u8>,
    pub palettes: Vec<u8>,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_without_args(cmd: &str) -> JsValue;
}

async fn request_data<T: DeserializeOwned>(cmd: &str) -> Result<T, ()> {
    let buffer = invoke_without_args(cmd).await;

    Ok(serde_wasm_bindgen::from_value::<T>(buffer).unwrap())
}

async fn request_frame() -> Result<Vec<u8>, ()> {
    let buffer = invoke_without_args("request_frame").await;
    let arr = Uint8Array::from(buffer);

    Ok(arr.to_vec())
}

#[function_component(EmulatorTauriWrapper)]
pub fn emulator_tauri_wrapper() -> Html {
    let fps_counter = use_mut_ref(FPSCounter::new);
    let fps = use_state_eq(|| Option::<usize>::None);

    let state = {
        let fps = fps.clone();

        use_async(async move {
            let result = request_frame().await;
            fps.set(Some(fps_counter.clone().borrow_mut().tick()));
            result
        })
    };

    {
        let state = state.clone();
        use_interval(
            move || {
                if !state.loading {
                    state.run();
                }
            },
            16,
        )
    }

    html! {
        <div>
            if let Some(frame) = &state.data {
                <Emulator frame={(frame).clone()} fps={*fps}/>
            }
        </div>
    }
}

#[function_component(NametablesTauriWrapper)]
pub fn nametables_tauri_wrapper() -> Html {
    let state = { use_async(async { request_data::<NametablesData>("request_nametables").await }) };

    {
        let state = state.clone();
        use_interval(
            move || {
                if !state.loading {
                    state.run();
                }
            },
            1000,
        )
    }

    html! {
        if let Some(data) = &state.data {
            <Nametables nametables={data.nametables.clone()} />
        }
    }
}

#[function_component(PPUTauriWrapper)]
pub fn ppu_tauri_wrapper() -> Html {
    let state = { use_async(async { request_data::<PPUData>("request_ppu").await }) };

    {
        let state = state.clone();
        use_interval(
            move || {
                if !state.loading {
                    state.run();
                }
            },
            1000,
        )
    }

    html! {
        if let Some(data) = &state.data {
            <PPU pattern_table_0={data.pattern_table_0.clone()} pattern_table_1={data.pattern_table_1.clone()} palettes={data.palettes.clone()}/>
        }
    }
}
