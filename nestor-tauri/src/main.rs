// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::{thread, time};

use nestor::frame::Frame;
use nestor::NES;
use tauri::api::dialog::FileDialogBuilder;
use tauri::{AboutMetadata, CustomMenuItem, Menu, MenuItem, State, Submenu};
use tauri::{Manager, Window};

struct Emulator(Arc<Mutex<NES>>);

impl Emulator {
    fn load_rom(&self, file_path: PathBuf) {
        let mut emulator = self.0.lock().unwrap();
        emulator.load_rom(file_path.into_os_string().into_string().unwrap());
        emulator.start_emulation();
    }

    fn emulate_frame(&self) -> Option<Frame> {
        let mut emulator = self.0.lock().unwrap();
        if let Some(frame) = emulator.emulate_frame() {
            Some(frame.clone())
        } else {
            None
        }
    }
}

#[tauri::command]
fn start_emulation(state: tauri::State<Emulator>, window: Window) {
    println!("start_emulation");

    let emulator = state.0.clone();
    let mut frames = 0.0;
    let mut now = time::Instant::now();
    // Spawn a new thread for emulation
    thread::spawn(move || {
        loop {
            // Lock the emulator, run one frame of emulation, and immediately unlock
            let mut emulator = emulator.lock().unwrap();
            let frame_data = emulator.emulate_frame();

            // Check if there is frame data to send to the UI
            if let Some(frame) = frame_data {
                frames += 1.0;

                let elapsed = now.elapsed();

                if elapsed.as_secs_f64() >= 1.0 {
                    println!("FPS: {}", frames);
                    frames = 0.0;
                    now = time::Instant::now();
                }

                // Send the frame data to the UI thread
                let mut local_buffer: Vec<u8> = vec![];

                for color in frame.data.chunks_exact(3) {
                    local_buffer.push(color[0]);
                    local_buffer.push(color[1]);
                    local_buffer.push(color[2]);
                    local_buffer.push(255);
                }

                window.emit("draw_frame", &local_buffer).unwrap();
            }
        }
    });
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
                        println!("Emulation ready to start");
                        window.emit("emulation_ready", "").unwrap();
                    }
                }),
            _ => {}
        })
        // .setup(|app| {
        //     #[cfg(debug_assertions)] // only include this code on debug builds
        //     {
        //         let window = app.get_window("main").unwrap();
        //         window.open_devtools();
        //         window.close_devtools();
        //     }
        //     Ok(())
        // })
        .invoke_handler(tauri::generate_handler![start_emulation])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
