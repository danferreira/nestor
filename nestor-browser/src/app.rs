use yew::prelude::*;
use yew_router::prelude::*;

use crate::tauri::{EmulatorTauriWrapper, NametablesTauriWrapper, PPUTauriWrapper};

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/tauri/emulator")]
    Emulator,
    #[at("/tauri/ppu")]
    PPU,
    #[at("/tauri/nametables")]
    Nametables,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! { {"Soon..."}},
        Route::Emulator => html! { <EmulatorTauriWrapper />},
        Route::PPU => html! {
            <PPUTauriWrapper />
        },
        Route::Nametables => html! { <NametablesTauriWrapper />},
    }
}

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} /> // <- must be child of <BrowserRouter>
        </BrowserRouter>
    }
}
