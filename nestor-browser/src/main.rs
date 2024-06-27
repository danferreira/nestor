use app::App;

mod app;
mod display;
mod nametables;
mod ppu;

fn main() {
    yew::Renderer::<App>::new().render();
}
