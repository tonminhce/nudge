use front_ui::icons::Icon;
use front_ui::icons::IconSearch;
use front_ui::icons::IconX;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use serde_json::Value;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::app::AuthState;
use crate::record_index::display_name;
use crate::record_index::primary_email;
use crate::record_index::text_filter;
use crate::records::load_object_metadata;
use crate::records::load_records;
use crate::records::records_from_response;

const COMMANDS: &[(&str, &str, &str, &str)] = &[
    ("People", "Open people records", "/objects/people", "users"),
    (
        "Companies",
        "Open company records",
        "/objects/companies",
        "building-skyscraper",
    ),
    (
        "Opportunities",
        "Open opportunities",
        "/objects/opportunities",
        "target-arrow",
    ),
    ("Tasks", "Open tasks", "/objects/tasks", "checkbox"),
    ("Notes", "Open notes", "/objects/notes", "notes"),
    (
        "Workflows",
        "Open workflows",
        "/workflows",
        "settings-automation",
    ),
    ("Agents", "Open agents", "/agents", "lego"),
    (
        "Settings",
        "Open workspace settings",
        "/settings",
        "settings",
    ),
    (
        "Add person",
        "Create a person",
        "/objects/people?create=person",
        "plus",
    ),
    (
        "Add company",
        "Create a company",
        "/objects/companies?create=company",
        "plus",
    ),
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct MenuItem {
    id: String,
    title: String,
    subtitle: String,
    href: String,
    icon: &'static str,
}

fn command_items(query: &str) -> Vec<MenuItem> {
    COMMANDS
        .iter()
        .filter(|(title, subtitle, href, _)| {
            query.is_empty()
                || title.to_lowercase().contains(query)
                || subtitle.to_lowercase().contains(query)
                || href.to_lowercase().contains(query)
        })
        .map(|(title, subtitle, href, icon)| MenuItem {
            id: format!("command:{href}"),
            title: (*title).to_string(),
            subtitle: (*subtitle).to_string(),
            href: (*href).to_string(),
            icon,
        })
        .collect()
}

async fn search_records(token: &str, query: &str) -> Result<Vec<(String, Value)>, String> {
    let objects = load_object_metadata(token).await?;
    let mut results = Vec::new();
    for singular in ["person", "company"] {
        let Some(object) = objects
            .iter()
            .find(|object| object.name_singular == singular)
            .cloned()
        else {
            continue;
        };
        let response = load_records(
            token,
            &object,
            None,
            text_filter(singular, query),
            None,
            1,
            false,
        )
        .await?;
        for record in records_from_response(&response, &object.name_plural)
            .into_iter()
            .take(5)
        {
            results.push((object.name_singular.clone(), record));
        }
    }
    Ok(results)
}

fn record_subtitle(singular: &str, record: &Value) -> String {
    if singular == "person" {
        primary_email(record)
    } else {
        record["domainName"]["primaryLinkUrl"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }
}

#[component]
pub fn CommandMenu() -> impl IntoView {
    let open = expect_context::<RwSignal<bool>>();
    let auth = expect_context::<AuthState>();
    let navigate = StoredValue::new_local(use_navigate());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let query = RwSignal::new(String::new());
    let record_results = RwSignal::new(Vec::<(String, Value)>::new());
    let searching = RwSignal::new(false);
    let search_error = RwSignal::new(None::<String>);
    let search_revision = RwSignal::new(0_usize);
    let selected_index = RwSignal::new(0_usize);

    let window = web_sys::window();
    let hotkey_store = StoredValue::new_local(Closure::<dyn FnMut(&KeyboardEvent)>::new(
        move |event: &KeyboardEvent| {
            let key = event.key().to_lowercase();
            if (event.meta_key() || event.ctrl_key()) && key == "k" {
                event.prevent_default();
                open.update(|open| *open = !*open);
            } else if key == "escape" && open.get_untracked() {
                event.prevent_default();
                open.set(false);
            }
        },
    ));
    if let Some(window) = window.as_ref() {
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
            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }
        }
    });

    let auth_for_search = auth.clone();
    Effect::new(move |_| {
        let is_open = open.get();
        let current_query = query.get();
        if !is_open {
            record_results.set(Vec::new());
            searching.set(false);
            return;
        }
        let current_query = current_query.trim().to_string();
        if current_query.len() < 2 {
            record_results.set(Vec::new());
            searching.set(false);
            search_error.set(None);
            return;
        }
        search_revision.update(|revision| *revision += 1);
        let revision = search_revision.get_untracked();
        let Some(token) = auth_for_search.token.get_untracked() else {
            return;
        };
        searching.set(true);
        search_error.set(None);
        leptos::task::spawn_local(async move {
            TimeoutFuture::new(200).await;
            let result = search_records(&token, &current_query).await;
            if search_revision.get_untracked() != revision {
                return;
            }
            match result {
                Ok(results) => {
                    record_results.set(results);
                    searching.set(false);
                }
                Err(message) => {
                    record_results.set(Vec::new());
                    searching.set(false);
                    search_error.set(Some(message));
                }
            }
        });
    });

    let items = Memo::new(move |_| {
        let current_query = query.get().to_lowercase();
        let mut items = command_items(&current_query);
        for (singular, record) in record_results.get() {
            let id = record["id"].as_str().unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            items.push(MenuItem {
                id: format!("record:{id}"),
                title: display_name(&record),
                subtitle: record_subtitle(&singular, &record),
                href: format!("/object/{singular}/{id}"),
                icon: if singular == "person" {
                    "user"
                } else {
                    "building-skyscraper"
                },
            });
        }
        items
    });

    let indexed_items = Memo::new(move |_| items.get().into_iter().enumerate().collect::<Vec<_>>());

    Effect::new(move |_| {
        let count = items.get().len();
        if selected_index.get_untracked() >= count {
            selected_index.set(count.saturating_sub(1));
        }
    });

    let run_selected = move || {
        let selected = selected_index.get_untracked();
        if let Some(item) = items.get().get(selected).cloned() {
            open.set(false);
            navigate.with_value(|navigate| navigate(&item.href, Default::default()));
        }
    };

    view! {
        <Show when=move || open.get() fallback=|| view! { <span /> }>
            <div class="command-menu-backdrop" on:click=move |_| open.set(false)>
                <aside class="command-menu-panel" role="dialog" aria-modal="true" aria-label="Command menu" on:click=|event| event.stop_propagation()>
                    <header class="command-menu-search">
                        <IconSearch size=18 />
                        <input
                            node_ref=input_ref
                            class="command-menu-input"
                            type="text"
                            placeholder="Search records or type a command..."
                            aria-label="Search records or commands"
                            bind:value=query
                            on:keydown=move |event| match event.key().as_str() {
                                "ArrowDown" => {
                                    event.prevent_default();
                                    let count = items.get().len();
                                    selected_index.update(|index| *index = if count == 0 { 0 } else { (*index + 1) % count });
                                }
                                "ArrowUp" => {
                                    event.prevent_default();
                                    let count = items.get().len();
                                    selected_index.update(|index| *index = if count == 0 { 0 } else { (*index + count - 1) % count });
                                }
                                "Enter" => {
                                    event.prevent_default();
                                    run_selected();
                                }
                                _ => {}
                            }
                        />
                        <button class="app-icon-button" aria-label="Close command menu" on:click=move |_| open.set(false)>
                            <IconX size=18 />
                        </button>
                    </header>
                    <div class="command-menu-content">
                        {move || if searching.get() {
                            view! { <div class="command-menu-status">"Searching records..."</div> }.into_any()
                        } else if let Some(message) = search_error.get() {
                            view! { <div class="record-error" role="alert">{message}</div> }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }}
                        <For each=move || indexed_items.get() key=|(_, item)| item.id.clone() let((index, item))>
                            {
                                let href = item.href.clone();
                                view! {
                                    <A
                                        href=href
                                        attr:class=move || if selected_index.get() == index { "command-menu-item selected" } else { "command-menu-item" }
                                        on:click=move |_| open.set(false)
                                    >
                                        <span class="command-menu-item-icon"><Icon name=item.icon size=16 /></span>
                                        <span class="command-menu-item-copy">
                                            <span class="command-menu-item-title">{item.title}</span>
                                            <span class="command-menu-item-subtitle">{item.subtitle}</span>
                                        </span>
                                        <span class="command-menu-item-enter">"↵"</span>
                                    </A>
                                }
                            }
                        </For>
                        {move || if items.get().is_empty() && !searching.get() {
                            view! { <div class="command-menu-empty">"No matching commands or records"</div> }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }}
                    </div>
                    <footer class="command-menu-footer">
                        <span>"↑↓ Navigate"</span><span>"↵ Open"</span><span>"esc Close"</span>
                    </footer>
                </aside>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_filter_matches_title_subtitle_and_route() {
        assert_eq!(command_items("").len(), COMMANDS.len());
        assert_eq!(command_items("company")[0].title, "Companies");
        assert!(command_items("create")
            .iter()
            .any(|item| item.title == "Add person"));
        assert!(command_items("settings")
            .iter()
            .any(|item| item.href == "/settings"));
    }
}
