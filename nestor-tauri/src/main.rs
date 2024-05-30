// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nestor::frame::Frame;
use nestor::NES;
use tauri::api::dialog::FileDialogBuilder;
use tauri::Manager;
use tauri::{AboutMetadata, CustomMenuItem, Menu, MenuItem, State, Submenu};

struct Emulator(Arc<Mutex<NES>>);

impl Emulator {
    fn load_rom(&self, file_path: PathBuf) {
        let mut emulator = self.0.lock().unwrap();
        emulator.load_rom(file_path.into_os_string().into_string().unwrap());
        emulator.start_emulation();
    }

    fn emulate_frame(&self) -> Option<Frame> {
        let mut emulator = self.0.lock().unwrap();
        if emulator.is_running() {
            if let Some(frame) = emulator.emulate_frame() {
                return Some(frame.clone());
            }
        }

        None
    }
}

#[tauri::command(async)]
fn next_frame(state: tauri::State<Emulator>) -> Vec<u8> {
    let emulator = state;
    let mut local_buffer: Vec<u8> = vec![];

    loop {
        // Check if there is frame data to return to the UI
        let frame_data = emulator.emulate_frame();
        if let Some(frame) = frame_data {
            for color in frame.data.chunks_exact(3) {
                local_buffer.push(color[0]);
                local_buffer.push(color[1]);
                local_buffer.push(color[2]);
                local_buffer.push(255);
            }

            break;
        }
    }
    return local_buffer;
}

fn main() {
    let app_menu = Submenu::new(
        "NEStor",
        Menu::new()
            .add_native_item(MenuItem::About(
                "NEStor".to_string(),
                AboutMetadata::default(),
            ))
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Services)
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Hide)
            .add_native_item(MenuItem::HideOthers)
            .add_native_item(MenuItem::ShowAll)
            .add_native_item(MenuItem::Separator)
            .add_native_item(MenuItem::Quit),
    );

    let open_rom = CustomMenuItem::new("open_rom".to_string(), "Open ROM");
    let file_menu = Submenu::new("File", Menu::new().add_item(open_rom));

    let ppu_viewer = CustomMenuItem::new("ppu_viewer".to_string(), "PPU Viewer");
    let nametable_viewer = CustomMenuItem::new("nametable_viewer".to_string(), "Nametable Viewer");
    let debug_menu = Submenu::new(
        "Debug",
        Menu::new().add_item(ppu_viewer).add_item(nametable_viewer),
    );

    let menu = Menu::new()
        .add_submenu(app_menu)
        .add_submenu(file_menu)
        .add_submenu(debug_menu);

    tauri::Builder::default()
        .manage(Emulator(Arc::new(Mutex::new(NES::new()))))
        .menu(menu)
        .on_menu_event(|event| match event.menu_item_id() {
            "open_rom" => FileDialogBuilder::new()
                .add_filter(".nes files", &["nes"])
                .pick_file(move |file_path| {
                    if let Some(file_path) = file_path {
                        let window = event.window();
                        let emulator: State<'_, Emulator> = window.state();
                        emulator.load_rom(file_path);
                    }
                }),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![next_frame])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
