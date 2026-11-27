use leptos::prelude::*;

mod app;
mod auth;
mod layout;
mod pages;

use app::App;

fn start() {
    mount_to_body(App);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_start() {
    start();
}

fn main() {
    start();
}
