use front_ui::icons::IconBell;
use front_ui::icons::IconBookmark;
use front_ui::icons::IconBuildingSkyscraper;
use front_ui::icons::IconCalendar;
use front_ui::icons::IconChevronDown;
use front_ui::icons::IconDots;
use front_ui::icons::IconHome;
use front_ui::icons::IconPlus;
use front_ui::icons::IconSearch;
use front_ui::icons::IconSettings;
use front_ui::icons::IconStar;
use front_ui::icons::IconUsers;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;

use crate::app::AuthState;
use crate::auth::ThemeToggle;

#[component]
pub fn WorkspaceLayout() -> impl IntoView {
    let auth = expect_context::<AuthState>();
    let navigate = use_navigate();
    let auth_for_effect = auth.clone();
    let navigate_for_effect = navigate.clone();
    Effect::new(move |_| {
        if !auth_for_effect.is_authenticated() {
            navigate_for_effect("/welcome", Default::default());
        }
    });
    let auth_for_show = auth.clone();
    let auth_for_signout = auth.clone();
    let navigate_for_signout = navigate.clone();
    let sign_out = move |_: web_sys::MouseEvent| {
        auth_for_signout.sign_out();
        navigate_for_signout("/welcome", Default::default());
    };

    view! {
        <Show
            when=move || auth_for_show.is_authenticated()
            fallback=|| view! { <div class="app-loading">Loading workspace...</div> }
        >
            <div class="workspace-shell">
                <header class="app-header">
                    <div class="header-brand">
                        <A href="/objects/people" attr:class="header-logo">"twenty"</A>
                        <span class="header-divider"></span>
                        <button class="workspace-switcher">
                            <span class="workspace-avatar">"R"</span>
                            <span>"Reference HQ"</span>
                            <IconChevronDown size=14 />
                        </button>
                    </div>
                    <button class="global-search">
                        <IconSearch size=16 />
                        <span>"Search"</span>
                        <kbd>"⌘ K"</kbd>
                    </button>
                    <div class="header-actions">
                        <button class="app-icon-button" aria-label="Notifications"><IconBell size=18 /></button>
                        <ThemeToggle />
                        <button class="user-avatar" aria-label="Sign out" on:click=sign_out.clone()>"M"</button>
                    </div>
                </header>
                <aside class="navigation-drawer">
                    <nav class="drawer-nav">
                        <A href="/getting-started" attr:class="drawer-item"><IconHome size=18 /><span>"Getting started"</span></A>
                        <div class="drawer-section-label">"Workspace"</div>
                        <A href="/objects/people" attr:class="drawer-item"><IconUsers size=18 /><span>"People"</span></A>
                        <A href="/objects/companies" attr:class="drawer-item"><IconBuildingSkyscraper size=18 /><span>"Companies"</span></A>
                        <A href="/objects/opportunities" attr:class="drawer-item"><IconStar size=18 /><span>"Opportunities"</span></A>
                        <A href="/objects/tasks" attr:class="drawer-item"><IconCalendar size=18 /><span>"Tasks"</span></A>
                        <A href="/objects/notes" attr:class="drawer-item"><IconBookmark size=18 /><span>"Notes"</span></A>
                        <div class="drawer-section-label">"Automations"</div>
                        <A href="/workflows" attr:class="drawer-item"><IconDots size=18 /><span>"Workflows"</span></A>
                        <A href="/agents" attr:class="drawer-item"><IconStar size=18 /><span>"Agents"</span></A>
                    </nav>
                    <div class="drawer-bottom">
                        <A href="/settings" attr:class="drawer-item"><IconSettings size=18 /><span>"Settings"</span></A>
                        <button class="drawer-item drawer-create"><IconPlus size=18 /><span>"Create new"</span></button>
                    </div>
                </aside>
                <main class="workspace-main"><Outlet /></main>
            </div>
        </Show>
    }
}
