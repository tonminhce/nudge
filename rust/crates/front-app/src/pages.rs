use front_core::client::GraphQLClient;
use front_core::projection::ObjectMetadata;
use front_ui::icons::IconBuildingSkyscraper;
use front_ui::icons::IconFilter;
use front_ui::icons::IconPlus;
use front_ui::icons::IconSearch;
use front_ui::icons::IconSettings;
use front_ui::icons::IconUsers;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use serde_json::Value;

use crate::app::AuthState;
use crate::records::end_cursor;
use crate::records::load_object_metadata;
use crate::records::load_records;
use crate::records::records_from_response;

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

    Effect::new(move |_| {
        let plural = params.get().get("plural").unwrap_or_default();
        let Some(token) = auth.token.get() else {
            return;
        };
        loading.set(true);
        error.set(None);
        records.set(Vec::new());
        cursor.set(None);
        leptos::task::spawn_local(async move {
            let result = async {
                let objects = load_object_metadata(&token).await?;
                let object = objects
                    .into_iter()
                    .find(|object| object.name_plural == plural)
                    .ok_or_else(|| format!("Object '{plural}' was not found"))?;
                let response = load_records(&token, &object, None).await?;
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

    let load_more = move |_: web_sys::MouseEvent| {
        let Some(token) = auth.token.get() else {
            return;
        };
        let Some(object) = metadata.get() else {
            return;
        };
        let after = cursor.get();
        loading.set(true);
        leptos::task::spawn_local(async move {
            match load_records(&token, &object, after).await {
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

    view! {
        <div class="record-index">
            <header class="record-index-header">
                <div class="record-title">{icon()}<h1>{title}</h1><span class="record-count">{move || total_count.get()}</span></div>
                <div class="record-actions">
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"><IconSearch size=16 />"Search"</button>
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"><IconFilter size=16 />"Filter"</button>
                    <button class="t-button" data-variant="solid" data-color="accent" data-size="md"><IconPlus size=16 />"Add " {title}</button>
                </div>
            </header>
            <div class="view-bar">
                <button class="view-tab active">"Table"</button>
                <button class="view-tab">"Kanban"</button>
                <button class="view-tab">"Calendar"</button>
                <button class="view-tab">"List"</button>
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
                    {move || records.get().into_iter().map(|record| {
                        let id = record["id"].as_str().unwrap_or_default().to_string();
                        let singular = singular();
                        view! {
                            <A href=format!("/object/{singular}/{id}") attr:class="record-table-row record-row-link" attr:role="row">
                                <span class="cell-checkbox"></span>
                                <span class="record-name">{display_name(&record)}</span>
                                <span>{record["companyId"].as_str().map(short_id).unwrap_or_default()}</span>
                                <span>{primary_email(&record)}</span>
                                <span>{created_at(&record)}</span>
                                <span>"→"</span>
                            </A>
                        }
                    }).collect::<Vec<_>>()}
                </div>
                <div class="record-pagination">
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        disabled=move || loading.get() || cursor.get().is_none() on:click=load_more>
                        {move || if loading.get() { "Loading..." } else { "Load more" }}
                    </button>
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn RecordShowPage() -> impl IntoView {
    let params = use_params_map();
    let auth = expect_context::<AuthState>();
    let record = RwSignal::new(None::<Value>);
    let singular_name = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let singular = params.get().get("singular").unwrap_or_default();
        let id = params.get().get("id").unwrap_or_default();
        let Some(token) = auth.token.get() else {
            return;
        };
        loading.set(true);
        error.set(None);
        leptos::task::spawn_local(async move {
            let result = async {
                let objects = load_object_metadata(&token).await?;
                let object = objects
                    .into_iter()
                    .find(|object| object.name_singular == singular)
                    .ok_or_else(|| format!("Object '{singular}' was not found"))?;
                let document = front_core::query_builder::find_many_query(
                    &object,
                    front_core::query_builder::FindManyArgs {
                        first: Some(1),
                        filter: Some(serde_json::json!({"id": {"eq": id}})),
                        ..Default::default()
                    },
                )
                .map_err(|error| error.to_string())?;
                let response = front_core::client::BrowserGraphQLClient::default()
                    .execute_data(
                        &crate::app::server_url(),
                        &document.query,
                        front_core::client::RequestOptions::new(front_core::client::Endpoint::Core)
                            .operation_name(format!("FindMany{}", object.name_plural))
                            .variables(Value::Object(document.variables))
                            .bearer_token(token),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                let records = records_from_response(&response, &object.name_plural);
                Ok::<_, String>((object.name_singular, records.into_iter().next()))
            }
            .await;
            match result {
                Ok((singular, Some(found))) => {
                    singular_name.set(singular);
                    record.set(Some(found));
                    loading.set(false);
                }
                Ok(_) => {
                    error.set(Some("Record not found".to_string()));
                    loading.set(false);
                }
                Err(message) => {
                    error.set(Some(message));
                    loading.set(false);
                }
            }
        });
    });

    view! {
        <div class="record-show">
            <header class="record-show-header">
                <A href="/objects/people" attr:class="breadcrumb">"People"</A>
                <span>" / "</span>
                <h1>{move || singular_name.get()}</h1>
            </header>
            {move || error.get().map(|message| view! { <div class="record-error" role="alert">{message}</div> })}
            <Show
                when=move || !loading.get() && record.get().is_some()
                fallback=move || view! {
                    <div class="record-loading">
                        {move || if loading.get() { "Loading record..." } else { "Record not found" }}
                    </div>
                }
            >
                {move || record.get().map(|record| view! {
                    <section class="record-show-card">
                        <h2>{display_name(&record)}</h2>
                        <dl class="record-details">
                            <dt>"Email"</dt><dd>{primary_email(&record)}</dd>
                            <dt>"Job title"</dt><dd>{record["jobTitle"].as_str().unwrap_or_default().to_string()}</dd>
                            <dt>"Company ID"</dt><dd>{record["companyId"].as_str().unwrap_or_default().to_string()}</dd>
                            <dt>"Created at"</dt><dd>{created_at(&record)}</dd>
                        </dl>
                    </section>
                })}
            </Show>
        </div>
    }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <div class="settings-page">
            <aside class="settings-nav">
                <h2>"Settings"</h2>
                <A href="/settings/profile" attr:class="">"Profile"</A>
                <A href="/settings/experience" attr:class="">"Experience"</A>
                <A href="/settings/objects" attr:class="">"Data model"</A>
                <A href="/settings/members" attr:class="">"Members"</A>
                <A href="/settings/roles" attr:class="">"Roles"</A>
                <A href="/settings/api-keys" attr:class="">"API keys"</A>
                <A href="/settings/general" attr:class="">"General"</A>
            </aside>
            <section class="settings-content">
                <IconSettings size=28 />
                <h1>"Workspace settings"</h1>
                <p>"Settings editors land in Wave 5 U5."</p>
            </section>
        </div>
    }
}

#[component]
pub fn StubPage(title: &'static str) -> impl IntoView {
    view! {
        <div class="stub-page">
            <h1>{title}</h1>
            <p>"This route is reserved by the Nudge-compatible application router."</p>
        </div>
    }
}

#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="not-found">
            <h1>"Page not found"</h1>
            <A href="/" attr:class="">"Return to workspace"</A>
        </div>
    }
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
