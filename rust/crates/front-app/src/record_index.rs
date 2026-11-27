use front_core::projection::ObjectMetadata;
use front_ui::icons::IconBuildingSkyscraper;
use front_ui::icons::IconFilter;
use front_ui::icons::IconPlus;
use front_ui::icons::IconSearch;
use front_ui::icons::IconUsers;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::app::AuthState;
use crate::records::end_cursor;
use crate::records::load_object_metadata;
use crate::records::load_records;
use crate::records::records_from_response;

pub const ROW_HEIGHT: usize = 32;
pub const ROW_SLOT_COUNT: usize = 240;
pub const OVERSCAN_ROWS: usize = 160;

#[derive(Clone, Copy, PartialEq, Eq)]
enum RecordSort {
    NameAsc,
    NameDesc,
    Newest,
    Oldest,
}

impl RecordSort {
    fn parse(value: &str) -> Self {
        match value {
            "nameDesc" => Self::NameDesc,
            "newest" => Self::Newest,
            "oldest" => Self::Oldest,
            _ => Self::NameAsc,
        }
    }
}

pub fn virtual_range(scroll_top: usize, viewport_height: usize, total: usize) -> (usize, usize) {
    let first = scroll_top / ROW_HEIGHT;
    let start = first.saturating_sub(OVERSCAN_ROWS).min(total);
    let end = (start + ROW_SLOT_COUNT).min(total);
    let _ = viewport_height;
    (start, end)
}

#[component]
pub fn RecordIndexPage() -> impl IntoView {
    let params = use_params_map();
    let auth = expect_context::<AuthState>();
    let records = RwSignal::new(Vec::<Value>::new());
    let metadata = RwSignal::new(None::<ObjectMetadata>);
    let cursor = RwSignal::new(None::<String>);
    let total_count = RwSignal::new(0_i64);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let search = RwSignal::new(String::new());
    let applied_search = RwSignal::new(String::new());
    let sort = RwSignal::new(RecordSort::NameAsc);
    let show_filters = RwSignal::new(false);
    let scroll_top = RwSignal::new(0_usize);
    let viewport_height = RwSignal::new(600_usize);

    Effect::new(move |_| {
        if let Some(window) = web_sys::window() {
            viewport_height.set(
                window
                    .inner_height()
                    .unwrap_or(JsValue::from(800.0))
                    .as_f64()
                    .unwrap_or(800.0) as usize
                    - 180,
            );
        }
    });

    Effect::new(move |_| {
        let plural = params.get().get("plural").unwrap_or_default();
        let Some(token) = auth.token.get() else {
            return;
        };
        let query = applied_search.get();
        let sort = sort.get();
        loading.set(true);
        error.set(None);
        records.set(Vec::new());
        cursor.set(None);
        scroll_top.set(0);
        leptos::task::spawn_local(async move {
            let result = async {
                let objects = load_object_metadata(&token).await?;
                let object = objects
                    .into_iter()
                    .find(|object| object.name_plural == plural)
                    .ok_or_else(|| format!("Object '{plural}' was not found"))?;
                let depth = if matches!(object.name_singular.as_str(), "person" | "company") {
                    1
                } else {
                    0
                };
                let response = load_records(
                    &token,
                    &object,
                    None,
                    text_filter(&object.name_singular, &query),
                    order_by(&object.name_singular, sort),
                    depth,
                )
                .await?;
                Ok::<_, String>((object, response))
            }
            .await;
            match result {
                Ok((object, response)) => {
                    total_count.set(
                        response[object.name_plural.as_str()]["totalCount"]
                            .as_i64()
                            .unwrap_or(0),
                    );
                    records.set(records_from_response(&response, &object.name_plural));
                    cursor.set(end_cursor(&response, &object.name_plural));
                    metadata.set(Some(object));
                    loading.set(false);
                }
                Err(message) => {
                    error.set(Some(message));
                    loading.set(false);
                }
            }
        });
    });

    let load_more = move || {
        let Some(token) = auth.token.get() else {
            return;
        };
        let Some(object) = metadata.get() else {
            return;
        };
        let Some(after) = cursor.get() else {
            return;
        };
        if loading.get() {
            return;
        }
        let depth = if matches!(object.name_singular.as_str(), "person" | "company") {
            1
        } else {
            0
        };
        loading.set(true);
        leptos::task::spawn_local(async move {
            let result = load_records(
                &token,
                &object,
                Some(after),
                text_filter(&object.name_singular, &applied_search.get()),
                order_by(&object.name_singular, sort.get()),
                depth,
            )
            .await;
            match result {
                Ok(response) => {
                    let mut next = records.get();
                    next.extend(records_from_response(&response, &object.name_plural));
                    records.set(next);
                    cursor.set(end_cursor(&response, &object.name_plural));
                    loading.set(false);
                }
                Err(message) => {
                    error.set(Some(message));
                    loading.set(false);
                }
            }
        });
    };
    let load_more_scroll = load_more;

    let plural = move || params.get().get("plural").unwrap_or_default();
    let singular = move || {
        metadata
            .get()
            .map(|object| object.name_singular)
            .unwrap_or_default()
    };
    let title = move || match plural().as_str() {
        "people" => "People".to_string(),
        "companies" => "Companies".to_string(),
        "opportunities" => "Opportunities".to_string(),
        "tasks" => "Tasks".to_string(),
        "notes" => "Notes".to_string(),
        other => other.to_string(),
    };
    let icon = move || -> AnyView {
        match plural().as_str() {
            "companies" => view! { <IconBuildingSkyscraper size=20 /> }.into_any(),
            _ => view! { <IconUsers size=20 /> }.into_any(),
        }
    };
    let rows = move || {
        let all = records.get();
        let (start, end) = virtual_range(scroll_top.get(), viewport_height.get(), all.len());
        all[start..end]
            .iter()
            .cloned()
            .enumerate()
            .map(|(offset, record)| (start + offset, record))
            .collect::<Vec<_>>()
    };

    view! {
        <div class="record-index">
            <header class="record-index-header">
                <div class="record-title">{icon()}<h1>{title}</h1><span class="record-count">{move || total_count.get()}</span></div>
                <div class="record-actions">
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        on:click=move |_| show_filters.set(!show_filters.get())>
                        <IconSearch size=16 />"Search"
                    </button>
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        on:click=move |_| show_filters.set(!show_filters.get())>
                        <IconFilter size=16 />"Filter"
                    </button>
                    <button class="t-button" data-variant="solid" data-color="accent" data-size="md" disabled=true>
                        <IconPlus size=16 />"Add " {title}
                    </button>
                </div>
            </header>
            <Show when=move || show_filters.get() fallback=|| view! { <span /> }>
                <form class="record-filter-bar" on:submit=move |event| {
                    event.prevent_default();
                    applied_search.set(search.get());
                }>
                    <IconSearch size=16 />
                    <input class="t-input" placeholder="Search by name or email" bind:value=search />
                    <select class="record-sort" on:change=move |event| {
                        let value = event
                            .target()
                            .and_then(|target| target.dyn_into::<web_sys::HtmlSelectElement>().ok())
                            .map(|select| select.value())
                            .unwrap_or_default();
                        sort.set(RecordSort::parse(&value));
                    }>
                        <option value="nameAsc" selected=true>"Name A-Z"</option>
                        <option value="nameDesc">"Name Z-A"</option>
                        <option value="newest">"Newest first"</option>
                        <option value="oldest">"Oldest first"</option>
                    </select>
                    <button class="t-button" type="submit" data-variant="solid" data-color="accent" data-size="md">"Apply"</button>
                </form>
            </Show>
            <div class="view-bar">
                <button class="view-tab active">"Table"</button>
                <button class="view-tab" disabled=true>"Kanban"</button>
                <button class="view-tab" disabled=true>"Calendar"</button>
                <button class="view-tab" disabled=true>"List"</button>
            </div>
            {move || error.get().map(|message| view! { <div class="record-error" role="alert">{message}</div> })}
            <Show
                when=move || !loading.get() && !records.get().is_empty()
                fallback=move || view! {
                    <div class="record-loading">
                        {move || if loading.get() { "Loading records..." } else { "No records found" }}
                    </div>
                }
            >
                <div class="record-table" role="table" aria-label="Records">
                    <div class="record-table-head" role="row">
                        <span class="cell-checkbox"></span><span>"Name"</span><span>"Company"</span><span>"Emails"</span><span>"Created at"</span><span></span>
                    </div>
                    <div class="record-table-body" role="rowgroup"
                        style=move || format!("height: {}px", records.get().len() * ROW_HEIGHT)
                        on:scroll=move |event| {
                            let Some(target) = event.target().and_then(|target| target.dyn_into::<web_sys::Element>().ok()) else {
                                return;
                            };
                            scroll_top.set(target.scroll_top() as usize);
                            viewport_height.set(target.client_height() as usize);
                            if target.scroll_top() + target.client_height() >= target.scroll_height() - 600 {
                                load_more_scroll();
                            }
                        }>
                        <For each=rows key=|(index, _)| *index let((index, record))>
                            <A href=format!("/object/{}/{}", singular(), record["id"].as_str().unwrap_or_default())
                                attr:class="record-table-row record-row-link" attr:role="row"
                                attr:style=format!("top: {}px", index * ROW_HEIGHT)>
                                <span class="cell-checkbox"></span>
                                <span class="record-name">{display_name(&record)}</span>
                                <span>{company_name(&record)}</span>
                                <span>{primary_email(&record)}</span>
                                <span>{created_at(&record)}</span>
                                <span>"→"</span>
                            </A>
                        </For>
                    </div>
                </div>
                <div class="record-pagination">
                    <span>{move || format!("Showing {} of {}", records.get().len(), total_count.get())}</span>
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        disabled=move || loading.get() || cursor.get().is_none() on:click=move |_| load_more()>
                        {move || if loading.get() { "Loading..." } else { "Load more" }}
                    </button>
                </div>
            </Show>
        </div>
    }
}

fn text_filter(object: &str, query: &str) -> Option<Value> {
    let query = query.trim();
    if query.is_empty() {
        return None;
    }
    let pattern = format!("%{query}%");
    if object == "person" {
        Some(serde_json::json!({
            "or": [
                {"name": {"firstName": {"ilike": pattern}}},
                {"name": {"lastName": {"ilike": pattern}}},
                {"emails": {"primaryEmail": {"ilike": pattern}}}
            ]
        }))
    } else if object == "company" {
        Some(serde_json::json!({"name": {"ilike": pattern}}))
    } else {
        None
    }
}

fn order_by(object: &str, sort: RecordSort) -> Option<Value> {
    let value = match (object, sort) {
        ("person", RecordSort::NameAsc) => {
            serde_json::json!([{"name": {"lastName": "AscNullsFirst"}}])
        }
        ("person", RecordSort::NameDesc) => {
            serde_json::json!([{"name": {"lastName": "DescNullsLast"}}])
        }
        (_, RecordSort::NameAsc) => serde_json::json!([{"name": "AscNullsFirst"}]),
        (_, RecordSort::NameDesc) => serde_json::json!([{"name": "DescNullsLast"}]),
        (_, RecordSort::Newest) => serde_json::json!([{"createdAt": "DescNullsLast"}]),
        (_, RecordSort::Oldest) => serde_json::json!([{"createdAt": "AscNullsFirst"}]),
    };
    Some(value)
}

fn display_name(record: &Value) -> String {
    let first = record["name"]["firstName"].as_str().unwrap_or_default();
    let last = record["name"]["lastName"].as_str().unwrap_or_default();
    match (first.is_empty(), last.is_empty()) {
        (true, true) => record["name"].as_str().unwrap_or_default().to_string(),
        (true, false) => last.to_string(),
        (false, true) => first.to_string(),
        (false, false) => format!("{first} {last}"),
    }
}

fn company_name(record: &Value) -> String {
    record["company"]["name"]
        .as_str()
        .map(str::to_string)
        .or_else(|| record["companyId"].as_str().map(short_id))
        .unwrap_or_default()
}

fn primary_email(record: &Value) -> String {
    record["emails"]["primaryEmail"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn created_at(record: &Value) -> String {
    record["createdAt"].as_str().unwrap_or_default().to_string()
}

fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_table_reuses_at_most_two_hundred_forty_slots() {
        let (start, end) = virtual_range(32_000, 600, 10_000);
        assert_eq!(start, 840);
        assert_eq!(end - start, ROW_SLOT_COUNT);
    }

    #[test]
    fn virtual_table_clamps_the_last_window_to_the_record_count() {
        let (start, end) = virtual_range(319_968, 600, 10_000);
        assert_eq!(start, 9_839);
        assert_eq!(end, 10_000);
    }

    #[test]
    fn person_search_uses_the_upstream_composite_filter_shape() {
        let filter = text_filter("person", "Ada").unwrap();
        assert_eq!(filter["or"][0]["name"]["firstName"]["ilike"], "%Ada%");
        assert_eq!(filter["or"][2]["emails"]["primaryEmail"]["ilike"], "%Ada%");
    }
}
