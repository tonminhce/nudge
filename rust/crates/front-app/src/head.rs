use front_core::client::BrowserGraphQLClient;
use front_core::client::Endpoint;
use front_core::client::GraphQLClient;
use front_core::client::RequestOptions;
use front_core::favicon::favicon_href;
use front_core::page_title::next_page_title_id;
use front_core::page_title::page_title_from_path;
use front_core::page_title::resolve_document_title;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::Title;
use leptos_router::hooks::use_location;
use serde_json::Value;
use wasm_bindgen::JsCast;

use crate::app::server_url;

#[derive(Clone, Copy)]
pub struct PageHead {
    pub route_title: RwSignal<String>,
    pub page_titles: RwSignal<Vec<(u64, String)>>,
    pub workspace_logo: RwSignal<Option<String>>,
}

impl PageHead {
    pub fn new() -> Self {
        Self {
            route_title: RwSignal::new(front_core::page_title::APP_NAME.to_string()),
            page_titles: RwSignal::new(Vec::new()),
            workspace_logo: RwSignal::new(None),
        }
    }

    pub fn provide(self) -> Self {
        provide_context(self);
        self
    }

    pub fn document_title(&self) -> String {
        resolve_document_title(
            &self.route_title.get_untracked(),
            &self.page_titles.get_untracked(),
        )
    }

    pub fn favicon_href(&self, base_url: &str) -> String {
        favicon_href(self.workspace_logo.get_untracked().as_deref(), base_url)
    }
}

impl Default for PageHead {
    fn default() -> Self {
        Self::new()
    }
}

#[component]
pub fn PageTitle(#[prop(into)] title: TextProp) -> impl IntoView {
    let head = expect_context::<PageHead>();
    let id = next_page_title_id();
    Effect::new(move |_| {
        let value = title.get();
        head.page_titles.update(|titles| {
            if let Some(slot) = titles.iter_mut().find(|(slot, _)| *slot == id) {
                slot.1 = value.to_string();
            } else {
                titles.push((id, value.to_string()));
            }
        });
    });
    on_cleanup(move || {
        head.page_titles
            .update(|titles| titles.retain(|(slot, _)| *slot != id));
    });
    ()
}

#[component]
pub fn RoutePageTitle() -> impl IntoView {
    let head = expect_context::<PageHead>();
    let location = use_location();
    Effect::new(move |_| {
        head.route_title
            .set(page_title_from_path(&location.pathname.get()));
    });
    view! {
        <Title text=move || resolve_document_title(&head.route_title.get(), &head.page_titles.get()) />
    }
}

fn favicon_element() -> Option<web_sys::Element> {
    let document = document();
    if let Some(existing) = document
        .query_selector("link[rel=\"icon\"]")
        .ok()
        .flatten()
        .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
    {
        return Some(existing);
    }
    let created = document.create_element("link").ok()?;
    let _ = created.set_attribute("rel", "icon");
    document.head()?.append_child(&created).ok()?;
    Some(created)
}

#[component]
pub fn PageFavicon() -> impl IntoView {
    let head = expect_context::<PageHead>();
    Effect::new(move |_| {
        let href = favicon_href(head.workspace_logo.get().as_deref(), &server_url());
        if let Some(link) = favicon_element() {
            let _ = link.set_attribute("type", "image/x-icon");
            let _ = link.set_attribute("href", &href);
            let _ = link.set_attribute("data-rh", "true");
        }
    });
    ()
}

fn current_origin() -> String {
    web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_default()
}

async fn fetch_workspace_logo(base_url: &str, origin: &str) -> Option<String> {
    let response = BrowserGraphQLClient::new()
        .execute_data(
            base_url,
            r#"query PublicWorkspaceData($origin: String!) { getPublicWorkspaceDataByDomain(origin: $origin) { id logo } }"#,
            RequestOptions::new(Endpoint::Metadata)
                .operation_name("PublicWorkspaceData")
                .variables(serde_json::json!({ "origin": Value::String(origin.to_string()) })),
        )
        .await
        .ok()?;
    response
        .get("getPublicWorkspaceDataByDomain")?
        .get("logo")?
        .as_str()
        .map(str::to_string)
}

#[component]
pub fn WorkspaceLogoEffect() -> impl IntoView {
    let head = expect_context::<PageHead>();
    Effect::new(move |_| {
        let base_url = server_url();
        let origin = current_origin();
        spawn_local(async move {
            head.workspace_logo
                .set(fetch_workspace_logo(&base_url, &origin).await);
        });
    });
    ()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_title_stack_falls_back_to_the_route_title() {
        let head = PageHead::new();
        head.route_title.set(String::from("Data model - Settings"));
        assert_eq!(head.document_title(), "Data model - Settings");
        head.page_titles.set(vec![
            (1, String::from("People")),
            (2, String::from("Opportunities")),
        ]);
        assert_eq!(head.document_title(), "Opportunities");
    }

    #[test]
    fn the_favicon_tracks_the_workspace_logo_and_falls_back_to_the_default() {
        let head = PageHead::new();
        assert_eq!(
            head.favicon_href("http://127.0.0.1:3011"),
            front_core::favicon::DEFAULT_WORKSPACE_LOGO
        );
        head.workspace_logo
            .set(Some(String::from("uploads/acme-logo.png")));
        assert_eq!(
            head.favicon_href("http://127.0.0.1:3011"),
            "http://127.0.0.1:3011/files/uploads/acme-logo.png"
        );
        head.workspace_logo.set(None);
        assert_eq!(
            head.favicon_href("http://127.0.0.1:3011"),
            front_core::favicon::DEFAULT_WORKSPACE_LOGO
        );
    }
}
