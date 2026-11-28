use front_core::client::GraphQLClient;
use front_ui::icons::Icon;
use front_ui::icons::IconSettings;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use leptos_router::hooks::use_query_map;
use serde_json::Value;
use wasm_bindgen::JsCast;

use crate::app::AuthState;
use crate::records::load_object_metadata;
use crate::records::records_from_response;
use crate::records::update_record;
use crate::side_panel::RecordSidePanel;

#[component]
pub fn RecordShowPage() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let auth = expect_context::<AuthState>();
    let record = RwSignal::new(None::<Value>);
    let object = RwSignal::new(None::<front_core::projection::ObjectMetadata>);
    let singular_name = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let editing = RwSignal::new(false);
    let show_side_panel = RwSignal::new(false);
    let draft_first_name = RwSignal::new(String::new());
    let draft_last_name = RwSignal::new(String::new());
    let draft_email = RwSignal::new(String::new());
    let draft_job_title = RwSignal::new(String::new());
    let draft_company_id = RwSignal::new(String::new());
    let companies = RwSignal::new(Vec::<Value>::new());
    let saving = RwSignal::new(false);
    let edit_error = RwSignal::new(None::<String>);
    let duplicates = RwSignal::new(Vec::<Value>::new());
    let duplicates_loading = RwSignal::new(false);
    let duplicates_error = RwSignal::new(None::<String>);
    let duplicates_searched = RwSignal::new(false);
    let record_generation = RwSignal::new(0_usize);
    let duplicate_generation = RwSignal::new(0_usize);

    Effect::new(move |_| {
        let singular = params.get().get("singular").unwrap_or_default();
        let id = params.get().get("id").unwrap_or_default();
        record_generation.set(record_generation.get_untracked() + 1);
        duplicate_generation.set(duplicate_generation.get_untracked() + 1);
        let generation = record_generation.get_untracked();
        duplicates.set(Vec::new());
        duplicates_loading.set(false);
        duplicates_error.set(None);
        duplicates_searched.set(false);
        editing.set(false);
        record.set(None);
        object.set(None);
        singular_name.set(String::new());
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
                        depth: 1,
                        ..Default::default()
                    },
                )
                .map_err(|error| error.to_string())?;
                let response = front_core::client::BrowserGraphQLClient::new()
                    .execute_data(
                        &crate::app::server_url(),
                        &document.query,
                        front_core::client::RequestOptions::new(front_core::client::Endpoint::Core)
                            .variables(Value::Object(document.variables))
                            .bearer_token(token),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                let records = records_from_response(&response, &object.name_plural);
                Ok::<_, String>((object.clone(), records.into_iter().next()))
            }
            .await;
            if record_generation.get_untracked() != generation {
                return;
            }
            match result {
                Ok((found_object, Some(found))) => {
                    let draft = RecordDraft::from_record(&found);
                    draft_first_name.set(draft.first_name);
                    draft_last_name.set(draft.last_name);
                    draft_email.set(draft.email);
                    draft_job_title.set(draft.job_title);
                    draft_company_id.set(draft.company_id);
                    singular_name.set(found_object.name_singular.clone());
                    object.set(Some(found_object));
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

    let start_edit = move || {
        if let Some(current) = record.get_untracked() {
            let draft = RecordDraft::from_record(&current);
            draft_first_name.set(draft.first_name);
            draft_last_name.set(draft.last_name);
            draft_email.set(draft.email);
            draft_job_title.set(draft.job_title);
            draft_company_id.set(draft.company_id);
        }
        editing.set(true);
        edit_error.set(None);
        let Some(token) = auth.token.get_untracked() else {
            return;
        };
        if !companies.get_untracked().is_empty() {
            return;
        }
        leptos::task::spawn_local(async move {
            let result = async {
                let objects = load_object_metadata(&token).await?;
                let company = objects
                    .into_iter()
                    .find(|object| object.name_singular == "company")
                    .ok_or_else(|| "Company metadata was not found".to_string())?;
                crate::records::load_records(&token, &company, None, None, None, 0, false).await
            }
            .await;
            match result {
                Ok(response) => {
                    companies.set(records_from_response(&response, "companies"));
                }
                Err(message) => edit_error.set(Some(message)),
            }
        });
    };

    Effect::new(move || {
        let query = query.get();
        if query.get("panel").as_deref() == Some("details") {
            show_side_panel.set(true);
        }
        if query.get("edit").as_deref() == Some("1") && singular_name.get() == "person" {
            start_edit();
        }
    });

    let save_edit = move || {
        let Some(token) = auth.token.get() else {
            return;
        };
        let Some(object) = object.get() else {
            return;
        };
        let Some(current_record) = record.get() else {
            return;
        };
        let id = current_record["id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let company_id = draft_company_id.get_untracked();
        let company = companies
            .get_untracked()
            .into_iter()
            .find(|company| company["id"].as_str() == Some(company_id.as_str()));
        let edit = RecordEdit {
            first_name: draft_first_name.get_untracked(),
            last_name: draft_last_name.get_untracked(),
            email: draft_email.get_untracked(),
            job_title: draft_job_title.get_untracked(),
            company_id,
            company,
        };
        saving.set(true);
        edit_error.set(None);
        leptos::task::spawn_local(async move {
            match update_record(
                &token,
                &object,
                &id,
                serde_json::json!({
                    "name": {"firstName": edit.first_name.clone(), "lastName": edit.last_name.clone()},
                    "emails": {"primaryEmail": edit.email.clone()},
                    "jobTitle": edit.job_title.clone(),
                    "companyId": if edit.company_id.is_empty() { Value::Null } else { Value::String(edit.company_id.clone()) }
                }),
            )
            .await
            {
                Ok(updated) => {
                    duplicate_generation.set(duplicate_generation.get_untracked() + 1);
                    duplicates.set(Vec::new());
                    duplicates_loading.set(false);
                    duplicates_error.set(None);
                    duplicates_searched.set(false);
                    record.set(Some(apply_record_edit(&current_record, &updated, &edit)));
                    saving.set(false);
                    editing.set(false);
                }
                Err(message) => {
                    saving.set(false);
                    edit_error.set(Some(message));
                }
            }
        });
    };

    let find_duplicates = move || {
        if loading.get_untracked() || duplicates_loading.get_untracked() {
            return;
        }
        let Some(token) = auth.token.get_untracked() else {
            return;
        };
        let Some(object) = object.get_untracked() else {
            return;
        };
        let Some(current_record) = record.get_untracked() else {
            return;
        };
        if object.duplicate_criteria.is_none() {
            return;
        }
        let id = current_record["id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let requested_singular = object.name_singular.clone();
        let requested_record_generation = record_generation.get_untracked();
        duplicate_generation.set(duplicate_generation.get_untracked() + 1);
        let generation = duplicate_generation.get_untracked();
        duplicates.set(Vec::new());
        duplicates_searched.set(false);
        duplicates_loading.set(true);
        duplicates_error.set(None);
        leptos::task::spawn_local(async move {
            let result = async {
                let document =
                    front_core::query_builder::find_duplicates_query(&object, vec![id.clone()])
                        .map_err(|error| error.to_string())?;
                let response = front_core::client::BrowserGraphQLClient::new()
                    .execute_data(
                        &crate::app::server_url(),
                        &document.query,
                        front_core::client::RequestOptions::new(front_core::client::Endpoint::Core)
                            .variables(Value::Object(document.variables))
                            .bearer_token(token),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                Ok::<_, String>(
                    response[format!("{}Duplicates", object.name_singular).as_str()]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .flat_map(|connection| connection["edges"].as_array().into_iter().flatten())
                        .filter_map(|edge| edge.get("node"))
                        .cloned()
                        .collect::<Vec<_>>(),
                )
            }
            .await;
            let current_id = record
                .get_untracked()
                .and_then(|record| record["id"].as_str().map(str::to_string));
            if duplicate_generation.get_untracked() != generation
                || record_generation.get_untracked() != requested_record_generation
                || current_id.as_deref() != Some(id.as_str())
                || singular_name.get_untracked() != requested_singular
            {
                return;
            }
            duplicates_searched.set(true);
            match result {
                Ok(found) => {
                    duplicates.set(found);
                    duplicates_loading.set(false);
                }
                Err(message) => {
                    duplicates.set(Vec::new());
                    duplicates_loading.set(false);
                    duplicates_error.set(Some(message));
                }
            }
        });
    };

    view! {
        <div class="record-show">
            <header class="record-show-header">
                <A href="/objects/people" attr:class="breadcrumb">"People"</A>
                <span>" / "</span>
                <h1>{move || singular_name.get()}</h1>
                <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                    aria-label="Open record details" on:click=move |_| show_side_panel.set(!show_side_panel.get())>
                    <Icon name="layout-sidebar-right" size=16 />
                </button>
                <Show when=move || singular_name.get() == "person" fallback=|| view! { <span /> }>
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        on:click=move |_| start_edit()>"Edit"</button>
                </Show>
                <Show when=move || object.get().and_then(|object| object.duplicate_criteria).is_some() fallback=|| view! { <span /> }>
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                        on:click=move |_| find_duplicates() disabled=move || loading.get() || duplicates_loading.get()>
                        {move || if duplicates_loading.get() { "Finding duplicates..." } else { "Find duplicates" }}
                    </button>
                </Show>
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
                            <dt>"Company"</dt><dd>{record["company"]["name"].as_str().unwrap_or_default().to_string()}</dd>
                            <dt>"Created at"</dt><dd>{created_at(&record)}</dd>
                        </dl>
                        <Show when=move || duplicates_loading.get() || duplicates_searched.get() || duplicates_error.get().is_some() || !duplicates.get().is_empty() fallback=|| view! { <span /> }>
                            <section class="record-duplicates" aria-live="polite">
                                <h3>"Duplicates"</h3>
                                {move || if duplicates_loading.get() {
                                    view! { <div class="record-loading">"Finding duplicates..."</div> }.into_any()
                                } else if let Some(message) = duplicates_error.get() {
                                    view! { <div class="record-error" role="alert">{message}</div> }.into_any()
                                } else if duplicates.get().is_empty() {
                                    view! { <div class="record-duplicates-empty">"No duplicates found"</div> }.into_any()
                                } else {
                                    view! {
                                        <div class="record-duplicates-list">
                                            <For each=move || duplicates.get() key=|record| record["id"].as_str().unwrap_or_default().to_string() let(record)>
                                                <A href=format!("/object/{}/{}", singular_name.get(), record["id"].as_str().unwrap_or_default())>
                                                    {display_name(&record)}
                                                </A>
                                            </For>
                                        </div>
                                    }.into_any()
                                }}
                            </section>
                        </Show>
                        <Show when=move || editing.get() && singular_name.get() == "person" fallback=|| view! { <span /> }>
                            <form class="record-edit-form" on:submit=move |event| {
                                event.prevent_default();
                                save_edit();
                            } on:keydown=move |event| {
                                if event.key() == "Enter" && (event.meta_key() || event.ctrl_key()) {
                                    event.prevent_default();
                                    save_edit();
                                }
                            }>
                                <div class="record-edit-grid">
                                    <label>"First name"<input class="t-input" bind:value=draft_first_name /></label>
                                    <label>"Last name"<input class="t-input" bind:value=draft_last_name /></label>
                                </div>
                                <label>"Email"<input class="t-input" type="email" bind:value=draft_email /></label>
                                <label>"Job title"<input class="t-input" bind:value=draft_job_title /></label>
                                <label>"Company"
                                    <select class="record-sort" on:change=move |event| {
                                        let value = event.target()
                                            .and_then(|target| target.dyn_into::<web_sys::HtmlSelectElement>().ok())
                                            .map(|select| select.value())
                                            .unwrap_or_default();
                                        draft_company_id.set(value);
                                    }>
                                        <option value="">"No company"</option>
                                        <For each=move || companies.get() key=|company| company["id"].as_str().unwrap_or_default().to_string() let(company)>
                                            {
                                                let id = company["id"].as_str().unwrap_or_default().to_string();
                                                let name = company["name"].as_str().unwrap_or_default().to_string();
                                                view! { <option value=id.clone() selected=move || draft_company_id.get() == id>{name}</option> }
                                            }
                                        </For>
                                    </select>
                                </label>
                                {move || edit_error.get().map(|message| view! { <div class="record-error" role="alert">{message}</div> })}
                                <div class="create-dialog-footer">
                                    <button type="button" class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                                        on:click=move |_| editing.set(false)>"Cancel"</button>
                                    <button type="submit" class="t-button" data-variant="solid" data-color="accent" data-size="md"
                                        disabled=move || saving.get()>
                                        {move || if saving.get() { "Saving..." } else { "Save" }}
                                    </button>
                                </div>
                            </form>
                        </Show>
                    </section>
                    <RecordSidePanel record=record.clone() open=show_side_panel />
                })}
            </Show>
        </div>
    }
}

struct RecordDraft {
    first_name: String,
    last_name: String,
    email: String,
    job_title: String,
    company_id: String,
}

impl RecordDraft {
    fn from_record(record: &Value) -> Self {
        Self {
            first_name: record["name"]["firstName"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            last_name: record["name"]["lastName"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            email: primary_email(record),
            job_title: record["jobTitle"].as_str().unwrap_or_default().to_string(),
            company_id: record["companyId"]
                .as_str()
                .or_else(|| record["company"]["id"].as_str())
                .unwrap_or_default()
                .to_string(),
        }
    }
}

struct RecordEdit {
    first_name: String,
    last_name: String,
    email: String,
    job_title: String,
    company_id: String,
    company: Option<Value>,
}

fn apply_record_edit(current: &Value, updated: &Value, edit: &RecordEdit) -> Value {
    let mut merged = current.as_object().cloned().unwrap_or_default();
    if let Some(fields) = updated.as_object() {
        for (field, value) in fields {
            merged.insert(field.clone(), value.clone());
        }
    }
    merged.insert(
        String::from("name"),
        serde_json::json!({"firstName": edit.first_name, "lastName": edit.last_name}),
    );
    merged.insert(
        String::from("emails"),
        serde_json::json!({"primaryEmail": edit.email}),
    );
    merged.insert(
        String::from("jobTitle"),
        Value::String(edit.job_title.clone()),
    );
    merged.insert(
        String::from("companyId"),
        if edit.company_id.is_empty() {
            Value::Null
        } else {
            Value::String(edit.company_id.clone())
        },
    );
    merged.insert(
        String::from("company"),
        edit.company.clone().unwrap_or(Value::Null),
    );
    Value::Object(merged)
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

pub(crate) fn primary_email(record: &Value) -> String {
    record["emails"]["primaryEmail"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn created_at(record: &Value) -> String {
    record["createdAt"].as_str().unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_edit_preserves_identity_and_updates_the_relation() {
        let current = serde_json::json!({
            "id": "person-1",
            "createdAt": "2026-11-27T09:00:00.000Z",
            "name": {"firstName": "Ada", "lastName": "Lovelace"},
            "emails": {"primaryEmail": "ada@example.com"},
            "jobTitle": "Engineer",
            "companyId": "company-1",
            "company": {"id": "company-1", "name": "Old company"}
        });
        let updated = serde_json::json!({"jobTitle": "Principal Engineer"});
        let company = serde_json::json!({"id": "company-2", "name": "New company"});
        let edit = RecordEdit {
            first_name: String::from("Ada"),
            last_name: String::from("Lovelace"),
            email: String::from("ada@new.example"),
            job_title: String::from("Principal Engineer"),
            company_id: String::from("company-2"),
            company: Some(company.clone()),
        };
        let record = apply_record_edit(&current, &updated, &edit);
        assert_eq!(record["id"], "person-1");
        assert_eq!(record["createdAt"], "2026-11-27T09:00:00.000Z");
        assert_eq!(record["name"]["firstName"], "Ada");
        assert_eq!(record["emails"]["primaryEmail"], "ada@new.example");
        assert_eq!(record["jobTitle"], "Principal Engineer");
        assert_eq!(record["companyId"], "company-2");
        assert_eq!(record["company"], company);
    }

    #[test]
    fn record_edit_can_clear_a_relation() {
        let current = serde_json::json!({"companyId": "company-1", "company": {"id": "company-1"}});
        let edit = RecordEdit {
            first_name: String::new(),
            last_name: String::new(),
            email: String::new(),
            job_title: String::new(),
            company_id: String::new(),
            company: None,
        };
        let record = apply_record_edit(&current, &serde_json::json!({}), &edit);
        assert!(record["companyId"].is_null());
        assert!(record["company"].is_null());
    }
}
