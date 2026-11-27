use front_ui::icons::IconX;
use leptos::prelude::*;
use serde_json::Value;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::pages::created_at;
use crate::pages::display_name;
use crate::pages::primary_email;

#[component]
pub fn RecordSidePanel(record: Value, open: RwSignal<bool>) -> impl IntoView {
    let close_ref = NodeRef::<leptos::html::Button>::new();
    let hotkey_store = StoredValue::new_local(Closure::<dyn FnMut(&KeyboardEvent)>::new(
        move |event: &KeyboardEvent| {
            if event.key() == "Escape" && open.get_untracked() {
                event.prevent_default();
                open.set(false);
            }
        },
    ));
    if let Some(window) = web_sys::window() {
        hotkey_store.with_value(|callback| {
            let _ = window
                .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref());
        });
    }
    on_cleanup(move || {
        if let Some(window) = web_sys::window() {
            hotkey_store.with_value(|callback| {
                let _ = window.remove_event_listener_with_callback(
                    "keydown",
                    callback.as_ref().unchecked_ref(),
                );
            });
        }
    });

    Effect::new(move |_| {
        if open.get() {
            if let Some(button) = close_ref.get() {
                let _ = button.focus();
            }
        }
    });

    let title = RwSignal::new(display_name(&record));
    let email = RwSignal::new(primary_email(&record));
    let job_title = RwSignal::new(record["jobTitle"].as_str().unwrap_or_default().to_string());
    let company = RwSignal::new(
        record["company"]["name"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    );
    let created = RwSignal::new(created_at(&record));
    let id = RwSignal::new(record["id"].as_str().unwrap_or_default().to_string());

    view! {
        <Show when=move || open.get() fallback=|| view! { <span /> }>
            <div class="side-panel-backdrop" on:click=move |_| open.set(false)>
                <aside class="record-side-panel" role="dialog" aria-modal="true" aria-label="Record details" on:click=|event| event.stop_propagation()>
                    <header class="record-side-panel-header">
                        <div>
                            <span class="record-side-panel-eyebrow">"Record details"</span>
                            <h2>{title.get()}</h2>
                        </div>
                        <button node_ref=close_ref class="app-icon-button" aria-label="Close record details" on:click=move |_| open.set(false)>
                            <IconX size=18 />
                        </button>
                    </header>
                    <div class="record-side-panel-content">
                        <section class="record-side-panel-section">
                            <h3>"Details"</h3>
                            <dl class="record-details">
                                <dt>"Email"</dt><dd>{email.get()}</dd>
                                <dt>"Job title"</dt><dd>{job_title.get()}</dd>
                                <dt>"Company"</dt><dd>{company.get()}</dd>
                                <dt>"Created at"</dt><dd>{created.get()}</dd>
                            </dl>
                        </section>
                        <section class="record-side-panel-section">
                            <h3>"Record"</h3>
                            <dl class="record-details">
                                <dt>"ID"</dt><dd class="record-side-panel-id">{id.get()}</dd>
                            </dl>
                        </section>
                    </div>
                </aside>
            </div>
        </Show>
    }
}
