use front_ui::icons::IconBuildingSkyscraper;
use front_ui::icons::IconFilter;
use front_ui::icons::IconPlus;
use front_ui::icons::IconSearch;
use front_ui::icons::IconSettings;
use front_ui::icons::IconUsers;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

#[component]
pub fn RecordIndexPage() -> impl IntoView {
    let params = use_params_map();
    let plural = move || params.get().get("plural").unwrap_or_default();
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
                <div class="record-title">{icon()}<h1>{title}</h1></div>
                <div class="record-actions">
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"><IconSearch size=16 />"Search"</button>
                    <button class="t-button" data-variant="outline" data-color="neutral" data-size="md"><IconFilter size=16 />"Filter"</button>
                    <button class="t-button" data-variant="solid" data-color="accent" data-size="md"><IconPlus size=16 />"Add {title}"</button>
                </div>
            </header>
            <div class="view-bar">
                <button class="view-tab active">"Table"</button>
                <button class="view-tab">"Kanban"</button>
                <button class="view-tab">"Calendar"</button>
                <button class="view-tab">"List"</button>
            </div>
            <div class="record-table" role="table" aria-label=title>
                <div class="record-table-head" role="row">
                    <span class="cell-checkbox"></span><span>"Name"</span><span>"Company"</span><span>"Emails"</span><span>"Created at"</span><span></span>
                </div>
                {(0..8).map(|_index| view! {
                    <div class="record-table-row" role="row">
                        <span class="cell-checkbox"></span>
                        <span class="skeleton skeleton-name"></span>
                        <span class="skeleton skeleton-cell"></span>
                        <span class="skeleton skeleton-cell"></span>
                        <span class="skeleton skeleton-date"></span>
                        <span></span>
                    </div>
                }).collect::<Vec<_>>()}
            </div>
            <footer class="record-index-footer">"Wave 5 U4 connects this surface to the live record API."</footer>
        </div>
    }
}

#[component]
pub fn RecordShowPage() -> impl IntoView {
    let params = use_params_map();
    let singular = move || params.get().get("singular").unwrap_or_default();
    let id = move || params.get().get("id").unwrap_or_default();
    view! {
        <div class="record-show">
            <header class="record-show-header">
                <A href="/objects/people" attr:class="breadcrumb">"People"</A>
                <span>" / "</span>
                <h1>{move || match singular().as_str() { "person" => "Person".to_string(), "company" => "Company".to_string(), other => other.to_string() }}</h1>
            </header>
            <section class="record-show-card">
                <div class="record-show-placeholder">{move || format!("{} · {}", singular(), id())}</div>
                <p>"Record show fields land in Wave 5 U4."</p>
            </section>
        </div>
    }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <div class="settings-page">
            <aside class="settings-nav">
                <h2>"Settings"</h2>
                <A href="/settings/profile">"Profile"</A>
                <A href="/settings/experience">"Experience"</A>
                <A href="/settings/objects">"Data model"</A>
                <A href="/settings/members">"Members"</A>
                <A href="/settings/roles">"Roles"</A>
                <A href="/settings/api-keys">"API keys"</A>
                <A href="/settings/general">"General"</A>
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
            <p>"This route is reserved by the Twenty-compatible application router."</p>
        </div>
    }
}

#[component]
pub fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="not-found">
            <h1>"Page not found"</h1>
            <A href="/">"Return to workspace"</A>
        </div>
    }
}
