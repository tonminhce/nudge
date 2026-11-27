use front_core::projection::ObjectMetadata;
use front_ui::icons::IconBuildingSkyscraper;
use front_ui::icons::IconCheck;
use front_ui::icons::IconChevronDown;
use front_ui::icons::IconFilter;
use front_ui::icons::IconPlus;
use front_ui::icons::IconSearch;
use front_ui::icons::IconUsers;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use leptos_router::hooks::use_query_map;
use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use crate::app::read_storage;
use crate::app::write_storage;
use crate::app::AuthState;
use crate::record_views::alternate_view;
use crate::record_views::view_type_icon;
use crate::record_views::RecordViewType;
use crate::records::create_record;
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

fn virtual_rows(
    records: &[Value],
    scroll_top: usize,
    viewport_height: usize,
) -> Vec<(usize, Value)> {
    let (start, end) = virtual_range(scroll_top, viewport_height, records.len());
    records[start..end]
        .iter()
        .cloned()
        .enumerate()
        .map(|(offset, record)| (start + offset, record))
        .collect()
}

#[component]
pub fn RecordIndexPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
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
    let show_views = RwSignal::new(false);
    let view_type = RwSignal::new(RecordViewType::Table);
    let show_create = RwSignal::new(false);
    let revision = RwSignal::new(0_usize);
    let first_name = RwSignal::new(String::new());
    let last_name = RwSignal::new(String::new());
    let record_name = RwSignal::new(String::new());
    let record_email = RwSignal::new(String::new());
    let record_job_title = RwSignal::new(String::new());
    let record_domain = RwSignal::new(String::new());
    let company_id = RwSignal::new(String::new());
    let companies = RwSignal::new(Vec::<Value>::new());
    let create_error = RwSignal::new(None::<String>);
    let creating = RwSignal::new(false);
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
        if let Some(stored) = read_storage(&format!("nudge.view.{plural}")) {
            view_type.set(RecordViewType::parse(&stored));
        }
        let Some(token) = auth.token.get() else {
            return;
        };
        let _ = revision.get();
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
                    records.update(|records| {
                        records.extend(records_from_response(&response, &object.name_plural));
                    });
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

    let open_create = move || {
        show_create.set(true);
        create_error.set(None);
        let Some(token) = auth.token.get() else {
            return;
        };
        if !companies.get().is_empty() {
            return;
        }
        leptos::task::spawn_local(async move {
            if let Ok(objects) = load_object_metadata(&token).await {
                if let Some(company) = objects
                    .into_iter()
                    .find(|object| object.name_singular == "company")
                {
                    if let Ok(response) = load_records(&token, &company, None, None, None, 0).await
                    {
                        companies.set(records_from_response(&response, &company.name_plural));
                    }
                }
            }
        });
    };

    Effect::new(move || {
        if query.get().get("create").is_some() {
            open_create();
        }
    });

    let submit_create = move || {
        let Some(token) = auth.token.get() else {
            return;
        };
        let Some(object) = metadata.get() else {
            return;
        };
        let mut data = serde_json::Map::new();
        match object.name_singular.as_str() {
            "person" => {
                data.insert(
                    String::from("name"),
                    serde_json::json!({"firstName": first_name.get(), "lastName": last_name.get()}),
                );
                if !record_email.get().trim().is_empty() {
                    data.insert(
                        String::from("emails"),
                        serde_json::json!({"primaryEmail": record_email.get()}),
                    );
                }
                if !record_job_title.get().trim().is_empty() {
                    data.insert(
                        String::from("jobTitle"),
                        Value::String(record_job_title.get()),
                    );
                }
                if !company_id.get().is_empty() {
                    data.insert(String::from("companyId"), Value::String(company_id.get()));
                }
            }
            "company" => {
                data.insert(String::from("name"), Value::String(record_name.get()));
                if !record_domain.get().trim().is_empty() {
                    data.insert(
                        String::from("domainName"),
                        serde_json::json!({"primaryLinkUrl": record_domain.get()}),
                    );
                }
            }
            _ => {
                data.insert(String::from("name"), Value::String(record_name.get()));
            }
        }
        creating.set(true);
        create_error.set(None);
        leptos::task::spawn_local(async move {
            match create_record(&token, &object, Value::Object(data)).await {
                Ok(_) => {
                    creating.set(false);
                    show_create.set(false);
                    first_name.set(String::new());
                    last_name.set(String::new());
                    record_name.set(String::new());
                    record_email.set(String::new());
                    record_job_title.set(String::new());
                    record_domain.set(String::new());
                    company_id.set(String::new());
                    revision.update(|revision| *revision += 1);
                }
                Err(message) => {
                    creating.set(false);
                    create_error.set(Some(message));
                }
            }
        });
    };

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
    let create_label = move || match plural().as_str() {
        "people" => "Add person",
        "companies" => "Add company",
        "opportunities" => "Add opportunity",
        "tasks" => "Add task",
        "notes" => "Add note",
        _ => "Add record",
    };
    let icon = move || -> AnyView {
        match plural().as_str() {
            "companies" => view! { <IconBuildingSkyscraper size=20 /> }.into_any(),
            _ => view! { <IconUsers size=20 /> }.into_any(),
        }
    };
    let rows = move || {
        records.with(|records| virtual_rows(records, scroll_top.get(), viewport_height.get()))
    };

    view! {
        <div class="record-index">
            <header class="record-index-header">
                <div class="record-title">{move || icon()}<h1>{title}</h1><span class="record-count">{move || total_count.get()}</span></div>
                <div class="record-actions">
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        on:click=move |_| show_filters.set(!show_filters.get())>
                        <IconSearch size=16 />"Search"
                    </button>
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        on:click=move |_| show_filters.set(!show_filters.get())>
                        <IconFilter size=16 />"Filter"
                    </button>
                    <button class="t-button" data-variant="solid" data-color="accent" data-size="md"
                        on:click=move |_| open_create()>
                        <IconPlus size=16 />{create_label}
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
            <div class="view-picker">
                <button class="view-picker-trigger" aria-expanded=move || if show_views.get() { "true" } else { "false" } on:click=move |_| show_views.set(!show_views.get())>
                    {move || view_type_icon(view_type.get())}
                    <span>"All"</span>
                    <span class="view-picker-count">{"· "}{move || total_count.get()}</span>
                    <IconChevronDown size=14 />
                </button>
                <Show when=move || show_views.get() fallback=|| view! { <span /> }>
                    <div class="view-picker-menu" role="menu" aria-label="View type">
                        <div class="view-picker-section">"View type"</div>
                        <For each=move || [RecordViewType::Table, RecordViewType::Kanban, RecordViewType::Calendar, RecordViewType::List] key=|kind| kind.as_str() let(kind)>
                            <button class="view-picker-option" role="menuitemradio" aria-checked=move || if view_type.get() == kind { "true" } else { "false" }
                                on:click=move |_| {
                                    view_type.set(kind);
                                    write_storage(&format!("nudge.view.{}", plural()), kind.as_str());
                                    show_views.set(false);
                                }>
                                {view_type_icon(kind)}
                                <span>{kind.label()}</span>
                                <Show when=move || view_type.get() == kind fallback=|| view! { <span /> }>
                                    <IconCheck size=14 />
                                </Show>
                            </button>
                        </For>
                    </div>
                </Show>
            </div>
            {move || error.get().map(|message| view! { <div class="record-error" role="alert">{message}</div> })}
            <Show
                when=move || !loading.get() && !records.with(|records| records.is_empty())
                fallback=move || view! {
                    <div class="record-loading">
                        {move || if loading.get() { "Loading records..." } else { "No records found" }}
                    </div>
                }
            >
                <Show when=move || view_type.get() == RecordViewType::Table fallback=move || alternate_view(view_type.get(), records.get(), singular())>
                <div class="record-table" role="table" aria-label="Records">
                    <div class="record-table-head" role="row">
                        <span class="cell-checkbox"></span><span>"Name"</span><span>"Company"</span><span>"Emails"</span><span>"Created at"</span><span></span>
                    </div>
                    <div class="record-table-body" role="rowgroup"
                        style=move || format!("height: {}px", records.with(|records| records.len()) * ROW_HEIGHT)
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
                            {
                                let record_id = record["id"].as_str().unwrap_or_default().to_string();
                                view! {
                                    <A href=move || format!("/object/{}/{}", singular(), record_id)
                                        attr:class="record-table-row record-row-link" attr:role="row"
                                        attr:style=format!("top: {}px", index * ROW_HEIGHT)>
                                        <span class="cell-checkbox"></span>
                                        <span class="record-name">{display_name(&record)}</span>
                                        <span>{company_name(&record)}</span>
                                        <span>{primary_email(&record)}</span>
                                        <span>{created_at(&record)}</span>
                                        <span>"→"</span>
                                    </A>
                                }
                            }
                        </For>
                    </div>
                </div>
                <div class="record-pagination">
                    <span>{move || format!("Showing {} of {}", records.with(|records| records.len()), total_count.get())}</span>
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        disabled=move || loading.get() || cursor.get().is_none() on:click=move |_| load_more()>
                        {move || if loading.get() { "Loading..." } else { "Load more" }}
                    </button>
                </div>
                </Show>
            </Show>
            <Show when=move || show_create.get() fallback=|| view! { <span /> }>
                <div class="modal-backdrop">
                    <section class="create-dialog" role="dialog" aria-modal="true" aria-label="Create record">
                        <header class="create-dialog-header">
                            <h2>{create_label}</h2>
                            <button class="app-icon-button" aria-label="Close" on:click=move |_| show_create.set(false)>"×"</button>
                        </header>
                        <form class="create-form" on:submit=move |event| {
                            event.prevent_default();
                            submit_create();
                        }>
                            {move || match singular().as_str() {
                                "person" => view! {
                                    <label>"First name"<input class="t-input" required bind:value=first_name /></label>
                                    <label>"Last name"<input class="t-input" required bind:value=last_name /></label>
                                    <label>"Email"<input class="t-input" type="email" bind:value=record_email /></label>
                                    <label>"Job title"<input class="t-input" bind:value=record_job_title /></label>
                                    <label>"Company"
                                        <select class="record-sort" on:change=move |event| {
                                            let value = event.target()
                                                .and_then(|target| target.dyn_into::<web_sys::HtmlSelectElement>().ok())
                                                .map(|select| select.value())
                                                .unwrap_or_default();
                                            company_id.set(value);
                                        }>
                                            <option value="">"No company"</option>
                                            <For each=move || companies.get() key=|company| company["id"].as_str().unwrap_or_default().to_string() let(company)>
                                                {
                                                    let id = company["id"].as_str().unwrap_or_default().to_string();
                                                    let name = company["name"].as_str().unwrap_or_default().to_string();
                                                    view! { <option value=id>{name}</option> }
                                                }
                                            </For>
                                        </select>
                                    </label>
                                }.into_any(),
                                "company" => view! {
                                    <label>"Name"<input class="t-input" required bind:value=record_name /></label>
                                    <label>"Domain"<input class="t-input" bind:value=record_domain placeholder="example.com" /></label>
                                }.into_any(),
                                _ => view! {
                                    <label>"Name"<input class="t-input" required bind:value=record_name /></label>
                                }.into_any(),
                            }}
                            {move || create_error.get().map(|message| view! { <div class="record-error" role="alert">{message}</div> })}
                            <footer class="create-dialog-footer">
                                <button type="button" class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                                    on:click=move |_| show_create.set(false)>"Cancel"</button>
                                <button type="submit" class="t-button" data-variant="solid" data-color="accent" data-size="md"
                                    disabled=move || creating.get()>
                                    {move || if creating.get() { "Creating..." } else { "Create" }}
                                </button>
                            </footer>
                        </form>
                    </section>
                </div>
            </Show>
        </div>
    }
}

pub(crate) fn text_filter(object: &str, query: &str) -> Option<Value> {
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

pub(crate) fn display_name(record: &Value) -> String {
    let first = record["name"]["firstName"].as_str().unwrap_or_default();
    let last = record["name"]["lastName"].as_str().unwrap_or_default();
    match (first.is_empty(), last.is_empty()) {
        (true, true) => record["name"].as_str().unwrap_or_default().to_string(),
        (true, false) => last.to_string(),
        (false, true) => first.to_string(),
        (false, false) => format!("{first} {last}"),
    }
}

pub(crate) fn company_name(record: &Value) -> String {
    record["company"]["name"]
        .as_str()
        .map(str::to_string)
        .or_else(|| record["companyId"].as_str().map(short_id))
        .unwrap_or_default()
}

pub(crate) fn primary_email(record: &Value) -> String {
    record["emails"]["primaryEmail"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn created_at(record: &Value) -> String {
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
    fn virtual_rows_preserve_the_visible_slice_and_original_indices() {
        let records = (0..10_000)
            .map(|index| serde_json::json!({"id": index}))
            .collect::<Vec<_>>();
        let rows = virtual_rows(&records, 300 * ROW_HEIGHT, 600);

        assert_eq!(rows.len(), ROW_SLOT_COUNT);
        for (offset, (index, record)) in rows.iter().enumerate() {
            let expected_index = 140 + offset;
            assert_eq!(*index, expected_index);
            assert_eq!(record, &records[expected_index]);
        }
    }

    #[test]
    fn person_search_uses_the_upstream_composite_filter_shape() {
        let filter = text_filter("person", "Ada").unwrap();
        assert_eq!(filter["or"][0]["name"]["firstName"]["ilike"], "%Ada%");
        assert_eq!(filter["or"][2]["emails"]["primaryEmail"]["ilike"], "%Ada%");
    }
}
