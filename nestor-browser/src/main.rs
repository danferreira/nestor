mod app;
mod emulator;
mod nametables;
mod ppu;
mod tauri;

use app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
