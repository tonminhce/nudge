use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use leptos::prelude::*;

mod app;
mod auth;
mod command_menu;
mod layout;
mod pages;
mod record_index;
mod record_views;
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

#[cfg(test)]
mod tests {
    #[test]
    fn upstream_motion_layer_is_bundled_with_reduced_motion() {
        let css = include_str!("../assets/animations.css");
        assert_eq!(css.matches("@keyframes").count(), 89);
        assert!(css.contains("@keyframes codicon-spin"));
        assert!(css.contains("@keyframes pulse"));
        assert!(css.contains("@keyframes _skeletonShimmer_17v1i_1"));
        assert!(css.contains("@keyframes sonner-fade-in"));
        assert!(css.contains("@keyframes ProseMirror-cursor-blink"));
        assert!(css.contains("prefers-reduced-motion"));
        assert!(css.contains("[data-animation=\"modal\"]"));
    }
}
