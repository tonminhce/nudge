use leptos::prelude::*;

use crate::components::button::join_classes;

pub const APP_HEADER_HEIGHT: u32 = 48;

pub const SIDE_PANEL_MIN_WIDTH: u32 = 320;
pub const SIDE_PANEL_MAX_WIDTH: u32 = 600;
pub const SIDE_PANEL_DEFAULT_WIDTH: u32 = 400;

pub const NAVIGATION_DRAWER_MIN_WIDTH: u32 = 180;
pub const NAVIGATION_DRAWER_MAX_WIDTH: u32 = 350;
pub const NAVIGATION_DRAWER_DEFAULT_WIDTH: u32 = 220;
pub const NAVIGATION_DRAWER_COLLAPSED_WIDTH: u32 = 48;

pub const MOBILE_VIEWPORT: u32 = 768;

pub fn side_panel_width_style(width: u32) -> String {
    format!("width: {width}px;")
}

pub const SIDE_PANEL_WRAPPER_CLASS: &str = "t-sidePanel-wrapper";
pub const SIDE_PANEL_CLASS: &str = "t-sidePanel";
pub const SIDE_PANEL_HEADER_CLASS: &str = "t-sidePanel-header";
pub const SIDE_PANEL_CONTENT_CLASS: &str = "t-sidePanel-content";
pub const NAVIGATION_DRAWER_CLASS: &str = "t-navigationDrawer";
pub const APP_HEADER_CLASS: &str = "t-appHeader";

#[component]
pub fn AppHeader(#[prop(optional)] children: Option<Children>) -> impl IntoView {
    let style =
        format!("block-size: {APP_HEADER_HEIGHT}px; min-block-size: {APP_HEADER_HEIGHT}px;");
    let slot = children.map(|c| c().into_any());
    view! {
        <header class=APP_HEADER_CLASS style=style>{slot}</header>
    }
}

#[component]
pub fn SidePanel(
    #[prop(optional)] open: bool,
    #[prop(optional)] width: Option<u32>,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let width = width.unwrap_or(SIDE_PANEL_DEFAULT_WIDTH);
    assert!(
        (SIDE_PANEL_MIN_WIDTH..=SIDE_PANEL_MAX_WIDTH).contains(&width),
        "side panel width {width} outside constraints"
    );
    let root_class = join_classes(&[
        Some(SIDE_PANEL_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let wrapper_style = if open {
        side_panel_width_style(width)
    } else {
        "width: 0px;".to_string()
    };
    let slot = children.map(|c| c().into_any());
    view! {
        <div class=SIDE_PANEL_WRAPPER_CLASS style=wrapper_style data-open=open.then_some("")>
            <aside class=root_class data-open=open.then_some("") style=side_panel_width_style(width)>
                <div
                    class=SIDE_PANEL_HEADER_CLASS
                    style=format!("block-size: {APP_HEADER_HEIGHT}px;")
                ></div>
                <div class=SIDE_PANEL_CONTENT_CLASS>{slot}</div>
            </aside>
        </div>
    }
}

#[component]
pub fn NavigationDrawer(
    #[prop(optional)] collapsed: bool,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let width = if collapsed {
        NAVIGATION_DRAWER_COLLAPSED_WIDTH
    } else {
        NAVIGATION_DRAWER_DEFAULT_WIDTH
    };
    let slot = children.map(|c| c().into_any());
    view! {
        <nav
            class=NAVIGATION_DRAWER_CLASS
            style=side_panel_width_style(width)
            data-collapsed=collapsed.then_some("")
            aria-label="Navigation"
        >{slot}</nav>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constraint_constants_match_upstream_files() {
        assert_eq!(APP_HEADER_HEIGHT, 48);
        assert_eq!(SIDE_PANEL_MIN_WIDTH, 320);
        assert_eq!(SIDE_PANEL_MAX_WIDTH, 600);
        assert_eq!(SIDE_PANEL_DEFAULT_WIDTH, 400);
        assert_eq!(NAVIGATION_DRAWER_MIN_WIDTH, 180);
        assert_eq!(NAVIGATION_DRAWER_MAX_WIDTH, 350);
        assert_eq!(NAVIGATION_DRAWER_DEFAULT_WIDTH, 220);
        assert_eq!(NAVIGATION_DRAWER_COLLAPSED_WIDTH, 48);
        assert_eq!(MOBILE_VIEWPORT, 768);
    }

    #[test]
    fn side_panel_open_and_closed_widths() {
        let out = (view! { <SidePanel open=true> {"body"} </SidePanel> })
            .into_view()
            .to_html();
        assert!(out.contains(SIDE_PANEL_WRAPPER_CLASS), "{out}");
        assert!(out.contains(SIDE_PANEL_CLASS), "{out}");
        assert!(out.contains("width: 400px"), "{out}");
        assert!(out.contains("data-open"), "{out}");
        assert!(out.contains(">body<"), "{out}");
        assert!(out.contains(SIDE_PANEL_HEADER_CLASS), "{out}");
        assert!(out.contains("block-size: 48px"), "{out}");
        let out = (view! { <SidePanel/> }).into_view().to_html();
        assert!(out.contains("width: 0px"), "{out}");
        assert!(!out.contains("data-open"), "{out}");
        assert!(
            out.contains("width: 400px"),
            "closed aside keeps its width token"
        );
    }

    #[test]
    fn side_panel_width_is_clamped_by_constraints() {
        let out = std::panic::catch_unwind(|| {
            (view! { <SidePanel open=true width=700/> })
                .into_view()
                .to_html()
        });
        assert!(out.is_err(), "width above max must panic");
        let out = std::panic::catch_unwind(|| {
            (view! { <SidePanel open=true width=100/> })
                .into_view()
                .to_html()
        });
        assert!(out.is_err(), "width below min must panic");
    }

    #[test]
    fn app_header_is_48px() {
        let out = (view! { <AppHeader> {"title"} </AppHeader> })
            .into_view()
            .to_html();
        assert!(out.contains("block-size: 48px"), "{out}");
        assert!(out.contains(">title<"), "{out}");
    }

    #[test]
    fn navigation_drawer_collapses_to_48() {
        let out = (view! { <NavigationDrawer/> }).into_view().to_html();
        assert!(out.contains("width: 220px"), "{out}");
        assert!(out.contains("aria-label=\"Navigation\""), "{out}");
        let out = (view! { <NavigationDrawer collapsed=true/> })
            .into_view()
            .to_html();
        assert!(out.contains("width: 48px"), "{out}");
        assert!(out.contains("data-collapsed"), "{out}");
    }
}
