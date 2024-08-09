use std::fs;

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    AppHandle, Emitter, Url, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_dialog::DialogExt;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn load_rom(app: AppHandle, path: String) {
    app.emit("load_rom", &path).unwrap();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
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

            app.set_menu(menu).unwrap();

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
                                app_clone.emit("load_rom", content).unwrap();
                            }
                        })
                } else if event.id() == "debug_ppu" {
                    WebviewWindowBuilder::new(
                        &app_clone,
                        "debug_ppu",
                        WebviewUrl::External(Url::parse("http://localhost:8080/ppu").unwrap()),
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
                            Url::parse("http://localhost:8080/nametables").unwrap(),
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
        .invoke_handler(tauri::generate_handler![load_rom])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
