use front_core::client::GraphQLClient;
use front_ui::icons::IconSettings;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use serde_json::Value;

use crate::app::AuthState;
use crate::records::load_object_metadata;
use crate::records::records_from_response;
use crate::records::update_record;

#[component]
pub fn RecordShowPage() -> impl IntoView {
    let params = use_params_map();
    let auth = expect_context::<AuthState>();
    let record = RwSignal::new(None::<Value>);
    let object = RwSignal::new(None::<front_core::projection::ObjectMetadata>);
    let singular_name = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let editing = RwSignal::new(false);
    let draft_job_title = RwSignal::new(String::new());
    let saving = RwSignal::new(false);
    let edit_error = RwSignal::new(None::<String>);

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
                        depth: 1,
                        ..Default::default()
                    },
                )
                .map_err(|error| error.to_string())?;
                let response = front_core::client::BrowserGraphQLClient::default()
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
            match result {
                Ok((found_object, Some(found))) => {
                    draft_job_title.set(found["jobTitle"].as_str().unwrap_or_default().to_string());
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
        let job_title = draft_job_title.get_untracked();
        saving.set(true);
        edit_error.set(None);
        leptos::task::spawn_local(async move {
            match update_record(
                &token,
                &object,
                &id,
                serde_json::json!({"jobTitle": job_title}),
            )
            .await
            {
                Ok(updated) => {
                    let mut merged = current_record.as_object().cloned().unwrap_or_default();
                    if let Some(fields) = updated.as_object() {
                        for (field, value) in fields {
                            merged.insert(field.clone(), value.clone());
                        }
                    }
                    record.set(Some(Value::Object(merged)));
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

    view! {
        <div class="record-show">
            <header class="record-show-header">
                <A href="/objects/people" attr:class="breadcrumb">"People"</A>
                <span>" / "</span>
                <h1>{move || singular_name.get()}</h1>
                <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"
                    on:click=move |_| editing.set(true)>"Edit"</button>
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
                        <Show when=move || editing.get() fallback=|| view! { <span /> }>
                            <form class="record-edit-form" on:submit=move |event| {
                                event.prevent_default();
                                save_edit();
                            }>
                                <label>"Job title"<input class="t-input" bind:value=draft_job_title /></label>
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
