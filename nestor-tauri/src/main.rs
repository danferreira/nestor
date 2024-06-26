// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // .setup(|app| {
        // let app_submenu = SubmenuBuilder::new(app, "NEStor")
        //     .item(&MenuItemBuilder::new("About").id("about").build(app)?)
        //     .build()?;
        // let file_submenu = SubmenuBuilder::new(app, "File")
        //     .item(&MenuItemBuilder::new("Open ROM").id("open_rom").build(app)?)
        //     .build()?;
        // let debugger_submenu = SubmenuBuilder::new(app, "Debugger")
        //     .items(&[
        //         &MenuItemBuilder::new("PPU Viewer")
        //             .id("ppu_viewer")
        //             .build(app)?,
        //         &MenuItemBuilder::new("Nametable Viewer")
        //             .id("nametable_viewer")
        //             .build(app)?,
        //     ])
        //     .build()?;
        // let menu = MenuBuilder::new(app)
        //     .item(&app_submenu)
        //     .item(&file_submenu)
        //     .item(&debugger_submenu)
        //     .build()?;
        // app.set_menu(menu)?;
        // app.on_menu_event(move |app, event| match event.id().as_ref() {
        //     "open_rom" => {
        //         // app.dialog()
        //         //     .file()
        //         //     .add_filter(".nes files", &["nes"])
        //         //     .pick_file(|file_path| {
        //         //         println!("{:?}", file_path);
        //         //     });
        //     }
        //     _ => {}
        // });
        //     Ok(())
        // })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
