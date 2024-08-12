use std::{fs, sync::Mutex};

use nestor::{NES, ROM};
use nestor_browser::{NametablesData, PPUData};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    Manager, State, Url, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
async fn request_frame(state: State<'_, Mutex<NES>>) -> Result<Vec<u8>, ()> {
    loop {
        let mut state = state.lock().unwrap();
        if state.is_running() {
            if let Some(frame) = state.emulate_frame() {
                return Ok(frame.to_rgba());
            }
        }
    }
}

#[tauri::command]
async fn request_ppu(state: State<'_, Mutex<NES>>) -> Result<PPUData, ()> {
    let state = state.lock().unwrap();
    let (pt_0, pt_1) = state.ppu_viewer();
    let palettes = state.palette_viewer();
    Ok(PPUData {
        pattern_table_0: pt_0.to_rgba(),
        pattern_table_1: pt_1.to_rgba(),
        palettes: palettes.to_rgba(),
    })
}

#[tauri::command]
async fn request_nametables(state: State<'_, Mutex<NES>>) -> Result<NametablesData, ()> {
    let state = state.lock().unwrap();
    let nametables = state.nametable_viewer();
    Ok(NametablesData {
        nametables: nametables.to_rgba(),
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let load_rom = MenuItemBuilder::with_id("load_rom", "Load ROM").build(app)?;

            let file_menu = SubmenuBuilder::new(app, "File")
                .items(&[&load_rom])
                .build()?;

            let debug_ppu = MenuItemBuilder::with_id("debug_ppu", "PPU").build(app)?;
            let debug_nametable =
                MenuItemBuilder::with_id("debug_nametable", "Nametables").build(app)?;

            let debug_menu = SubmenuBuilder::new(app, "Debug")
                .items(&[&debug_ppu, &debug_nametable])
                .build()?;

            let menu = MenuBuilder::new(app)
                .items(&[
                    &PredefinedMenuItem::separator(app)?,
                    &file_menu,
                    &debug_menu,
                ])
                .build()?;

            app.manage(Mutex::new(NES::new()));

            app.set_menu(menu).unwrap();

            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            app.on_menu_event(move |app, event| {
                let app_clone = app.clone();
                if event.id() == "load_rom" {
                    app.dialog()
                        .file()
                        .add_filter("ROM", &["nes"])
                        .pick_file(move |file_path| {
                            if let Some(file_response) = file_path {
                                let content =
                                    fs::read(file_response.path).expect("Failed to read ROM file");
                                let state = app_clone.state::<Mutex<NES>>();
                                let rom = ROM::from_bytes(&content).unwrap();
                                state.lock().unwrap().insert_cartridge(rom);
                            }
                        })
                } else if event.id() == "debug_ppu" {
                    WebviewWindowBuilder::new(
                        &app_clone,
                        "debug_ppu",
                        WebviewUrl::External(
                            Url::parse("http://localhost:8080/tauri/ppu").unwrap(),
                        ),
                    )
                    .title("NEStor - PPU Debug")
                    .inner_size(600.0, 600.0)
                    .build()
                    .unwrap();
                } else if event.id() == "debug_nametable" {
                    WebviewWindowBuilder::new(
                        &app_clone,
                        "debug_nametable",
                        WebviewUrl::External(
                            Url::parse("http://localhost:8080/tauri/nametables").unwrap(),
                        ),
                    )
                    .title("NEStor - Nametables Debug")
                    .inner_size(600.0, 600.0)
                    .build()
                    .unwrap();
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            request_frame,
            request_ppu,
            request_nametables
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
