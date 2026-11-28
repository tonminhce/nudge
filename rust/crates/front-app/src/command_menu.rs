use front_core::query_builder::SearchQueryArgs;
use front_core::search::load_search_records;
use front_core::search::SearchRecord;
use front_core::search::MAX_SEARCH_RESULTS;
use front_ui::icons::Icon;
use front_ui::icons::IconSearch;
use front_ui::icons::IconX;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;
use web_sys::KeyboardEvent;

use crate::app::server_url;
use crate::app::AuthState;

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
    image_url: Option<String>,
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
            image_url: None,
        })
        .collect()
}

fn record_menu_item(record: SearchRecord) -> Option<MenuItem> {
    if record.record_id.is_empty() {
        return None;
    }
    let singular = record.object_name_singular;
    let image_url = record.image_url;
    Some(MenuItem {
        id: format!("record:{singular}:{}", record.record_id),
        title: record.label,
        subtitle: record.object_label_singular,
        href: format!("/object/{singular}/{}", record.record_id),
        icon: match singular.as_str() {
            "person" => "user",
            "company" => "building-skyscraper",
            "opportunity" => "target-arrow",
            "task" => "checkbox",
            "note" => "notes",
            _ => "file",
        },
        image_url,
    })
}

fn is_editable_event_target(event: &KeyboardEvent) -> bool {
    event
        .target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
        .map(|element| {
            matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || element.is_content_editable()
        })
        .unwrap_or(false)
}

#[component]
pub fn CommandMenu() -> impl IntoView {
    let open = expect_context::<RwSignal<bool>>();
    let auth = expect_context::<AuthState>();
    let navigate = StoredValue::new_local(use_navigate());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let query = RwSignal::new(String::new());
    let record_results = RwSignal::new(Vec::<MenuItem>::new());
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
            } else if key == "/" && !open.get_untracked() && !is_editable_event_target(event) {
                event.prevent_default();
                open.set(true);
            } else if key == "escape" && open.get_untracked() {
                event.prevent_default();
                if query.get_untracked().is_empty() {
                    open.set(false);
                } else {
                    query.set(String::new());
                }
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
        search_revision.update(|revision| *revision += 1);
        let revision = search_revision.get_untracked();
        if !is_open {
            record_results.set(Vec::new());
            searching.set(false);
            search_error.set(None);
            return;
        }
        let current_query = current_query.trim().to_string();
        if current_query.len() < 2 {
            record_results.set(Vec::new());
            searching.set(false);
            search_error.set(None);
            selected_index.set(0);
            return;
        }
        selected_index.set(0);
        let Some(token) = auth_for_search.token.get_untracked() else {
            record_results.set(Vec::new());
            searching.set(false);
            return;
        };
        let base_url = server_url();
        searching.set(true);
        search_error.set(None);
        leptos::task::spawn_local(async move {
            TimeoutFuture::new(200).await;
            let result = load_search_records(
                &base_url,
                &token,
                SearchQueryArgs {
                    search_input: current_query,
                    limit: MAX_SEARCH_RESULTS,
                    after: None,
                    included_object_name_singulars: None,
                    excluded_object_name_singulars: None,
                    filter: None,
                },
            )
            .await
            .map(|connection| {
                connection
                    .edges
                    .into_iter()
                    .filter_map(|edge| record_menu_item(edge.node))
                    .collect::<Vec<_>>()
            })
            .map_err(|error| error.to_string());
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
        let mut items = command_items(&query.get().to_lowercase());
        items.extend(record_results.get());
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
                            aria-autocomplete="list"
                            aria-controls="command-menu-results"
                            attr:aria-activedescendant=move || indexed_items.get().get(selected_index.get()).map(|(_, item)| item.id.clone())
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
                                "Escape" => {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    if query.get_untracked().is_empty() {
                                        open.set(false);
                                    } else {
                                        query.set(String::new());
                                    }
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
                            view! { <div class="command-menu-status" role="status" aria-live="polite">"Searching records..."</div> }.into_any()
                        } else if let Some(message) = search_error.get() {
                            view! { <div class="record-error" role="alert">{message}</div> }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }}
                        <div id="command-menu-results" role="listbox" aria-label="Commands and records">
                        <For each=move || indexed_items.get() key=|(_, item)| item.id.clone() let((index, item))>
                            {
                                let href = item.href.clone();
                                let image_url = item.image_url.clone();
                                let icon = item.icon;
                                view! {
                                    <A
                                        attr:id=item.id.clone()
                                        href=href
                                        attr:role="option"
                                        attr:aria-selected=move || selected_index.get() == index
                                        attr:class=move || if selected_index.get() == index { "command-menu-item selected" } else { "command-menu-item" }
                                        on:click=move |_| open.set(false)
                                    >
                                        <span class="command-menu-item-icon">
                                            {move || if let Some(url) = image_url.as_ref().filter(|url| !url.is_empty()) {
                                                view! { <img class="command-menu-item-avatar" src=url.clone() alt="" /> }.into_any()
                                            } else {
                                                view! { <Icon name=icon size=16 /> }.into_any()
                                            }}
                                        </span>
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

    #[test]
    fn search_record_maps_to_the_upstream_object_route() {
        let item = record_menu_item(SearchRecord {
            record_id: String::from("company-1"),
            object_name_singular: String::from("company"),
            object_label_singular: String::from("Company"),
            label: String::from("Visible Company A"),
            image_url: None,
            ts_rank_cd: 0.1,
            ts_rank: 0.06079271,
        })
        .expect("record menu item");

        assert_eq!(item.id, "record:company:company-1");
        assert_eq!(item.title, "Visible Company A");
        assert_eq!(item.subtitle, "Company");
        assert_eq!(item.href, "/object/company/company-1");
        assert_eq!(item.icon, "building-skyscraper");
    }
}
