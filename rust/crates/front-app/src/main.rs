use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use leptos::prelude::*;

mod app;
mod auth;
mod layout;
mod pages;
mod record_index;
mod records;

use app::App;

static STARTED: AtomicBool = AtomicBool::new(false);

fn start() {
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
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
