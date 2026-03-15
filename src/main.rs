use wasm_bindgen::prelude::*;
use log::Level;

mod app;
mod audio;
mod components;
mod export;
mod patterns;
mod presets;

#[wasm_bindgen(start)]
pub fn run_app() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Debug).expect("Failed to initialize logger");

    yew::Renderer::<app::App>::new().render();
    Ok(())
}

fn main() {} 