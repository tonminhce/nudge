use front_ui::gallery::gallery_page;
use front_ui::tokens::Scheme;
use leptos::prelude::*;
use leptos_meta::provide_meta_context;
use leptos_meta::Title;
use leptos_router::components::ParentRoute;
use leptos_router::components::Redirect;
use leptos_router::components::Route;
use leptos_router::components::Router;
use leptos_router::components::Routes;
use leptos_router::path;
use serde_json::Value;

use crate::auth::AuthPage;
use crate::command_menu::CommandMenu;
use crate::layout::WorkspaceLayout;
use crate::pages::NotFoundPage;
use crate::pages::RecordShowPage;
use crate::pages::SettingsPage;
use crate::pages::StubPage;
use crate::record_index::RecordIndexPage;

#[derive(Clone)]
pub struct AuthState {
    pub token: RwSignal<Option<String>>,
    pub workspace_id: RwSignal<Option<String>>,
    pub user: RwSignal<Option<Value>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            token: RwSignal::new(read_storage("nudge.token")),
            workspace_id: RwSignal::new(read_storage("nudge.workspaceId")),
            user: RwSignal::new(None),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.token.get().is_some()
    }

    pub fn sign_out(&self) {
        self.token.set(None);
        self.workspace_id.set(None);
        self.user.set(None);
        remove_storage("nudge.token");
        remove_storage("nudge.workspaceId");
    }
}

impl Default for AuthState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn read_storage(key: &str) -> Option<String> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
}

pub fn write_storage(key: &str, value: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(key, value);
    }
}

pub fn remove_storage(key: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.remove_item(key);
    }
}

pub fn server_url() -> String {
    read_storage("nudge.serverUrl").unwrap_or_else(|| "http://127.0.0.1:3000".to_string())
}

#[component]
pub fn App() -> impl IntoView {
    let auth = AuthState::new();
    let theme = RwSignal::new(
        read_storage("nudge.theme")
            .map(|theme| {
                if theme == "dark" {
                    Scheme::Dark
                } else {
                    Scheme::Light
                }
            })
            .unwrap_or(Scheme::Light),
    );
    provide_meta_context();
    provide_context(auth.clone());
    provide_context(theme);
    provide_context(RwSignal::new(false));

    Effect::new(move |_| {
        let class = match theme.get() {
            Scheme::Light => "light",
            Scheme::Dark => "dark",
        };
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            if let Some(root) = document.document_element() {
                root.set_class_name(class);
                let _ = root.set_attribute("style", &format!("color-scheme: {class};"));
            }
        }
    });

    view! {
        <Router>
            <Routes fallback=NotFoundPage>
                <Route path=path!("/welcome") view=AuthPage />
                <Route path=path!("/verify-email") view=|| view! { <StubPage title="Verify email" /> } />
                <Route path=path!("/reset-password/:token") view=|| view! { <StubPage title="Reset password" /> } />
                <Route path=path!("/invite/:hash") view=|| view! { <StubPage title="Join workspace" /> } />
                <Route path=path!("/workspace-activation") view=|| view! { <StubPage title="Activate workspace" /> } />
                <Route path=path!("/create/profile") view=|| view! { <StubPage title="Create profile" /> } />
                <Route path=path!("/sync/emails") view=|| view! { <StubPage title="Sync emails" /> } />
                <Route path=path!("/invite-team") view=|| view! { <StubPage title="Invite team" /> } />
                <Route path=path!("/book-call") view=|| view! { <StubPage title="Book a call" /> } />
                <Route path=path!("/plan-required") view=|| view! { <StubPage title="Plan required" /> } />
                <Route path=path!("/authorize") view=|| view! { <StubPage title="Authorize" /> } />
                <Route path=path!("/gallery") view=GalleryPage />
                <ParentRoute path=path!("") view=WorkspaceLayout>
                    <Route path=path!("") view=IndexRedirect />
                    <Route path=path!("/objects/:plural") view=RecordIndexPage />
                    <Route path=path!("/object/:singular/:id") view=RecordShowPage />
                    <Route path=path!("/settings/*rest") view=SettingsPage />
                    <Route path=path!("/getting-started") view=|| view! { <StubPage title="Getting started" /> } />
                    <Route path=path!("/workflows") view=|| view! { <StubPage title="Workflows" /> } />
                    <Route path=path!("/workflow/:id") view=|| view! { <StubPage title="Workflow" /> } />
                    <Route path=path!("/agents") view=|| view! { <StubPage title="Agents" /> } />
                    <Route path=path!("/agent/:id") view=|| view! { <StubPage title="Agent" /> } />
                    <Route path=path!("/chat/*rest") view=|| view! { <StubPage title="Chat" /> } />
                    <Route path=path!("/inbox/*rest") view=|| view! { <StubPage title="Inbox" /> } />
                    <Route path=path!("/home") view=|| view! { <StubPage title="Home" /> } />
                    <Route path=path!("/page/:id") view=|| view! { <StubPage title="Page" /> } />
                    <Route path=path!("/dpa") view=|| view! { <StubPage title="Data processing" /> } />
                    <Route path=path!("*rest") view=NotFoundPage />
                </ParentRoute>
                <Route path=path!("*rest") view=NotFoundPage />
            </Routes>
        </Router>
        <CommandMenu />
    }
}

#[component]
fn GalleryPage() -> impl IntoView {
    view! {
        <>
            <Title text="Nudge front-ui gallery" />
            <header class="record-index-header">
                <div class="record-title">
                    <h1>{"Nudge front-ui gallery"}</h1>
                </div>
                <div class="record-actions">
                    <a href="/" class="t-button">{"Back to app"}</a>
                </div>
            </header>
            {gallery_page()}
        </>
    }
}

#[component]
fn IndexRedirect() -> impl IntoView {
    view! { <Redirect path="/objects/people" /> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_route_renders_title_link_and_both_themes() {
        let html = GalleryPage().into_view().to_html();
        assert!(html.contains("<h1>Nudge front-ui gallery</h1>"));
        assert!(html.contains("href=\"/\""));
        assert!(html.contains(">Back to app<"));
        assert!(html.contains("class=\"light gallery\""));
        assert!(html.contains("class=\"dark gallery\""));
        assert!(html.contains(">Accent solid<"));
    }
}
