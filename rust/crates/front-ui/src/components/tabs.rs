use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TabSize {
    Sm,
    #[default]
    Md,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TabVariant {
    #[default]
    Underlined,
    Filled,
}

impl TabSize {
    pub fn as_str(self) -> &'static str {
        match self {
            TabSize::Sm => "sm",
            TabSize::Md => "md",
        }
    }
}

impl TabVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            TabVariant::Underlined => "underlined",
            TabVariant::Filled => "filled",
        }
    }
}

pub const TABS_ROOT_CLASS: &str = "t-tabs-root";
pub const TABS_LIST_CLASS: &str = "t-tabs-list";
pub const TABS_TAB_CLASS: &str = "t-tabs-tab";
pub const TABS_CONTENT_CLASS: &str = "t-tabs-content";
pub const TABS_LABEL_CLASS: &str = "t-tabs-label";
pub const TABS_START_ICON_CLASS: &str = "t-tabs-startIcon";
pub const TABS_PANEL_CLASS: &str = "t-tabs-panel";
pub const TABS_INDICATOR_CLASS: &str = "t-tabs-indicator";
pub const TAB_BUTTON_CLASS: &str = "t-tabButton-button";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TabItem {
    pub value: String,
    pub label: String,
}

impl TabItem {
    pub fn new(value: &str, label: &str) -> Self {
        Self {
            value: value.to_string(),
            label: label.to_string(),
        }
    }
}

#[component]
pub fn TabList(
    tabs: Vec<TabItem>,
    #[prop(optional)] value: &'static str,
    #[prop(optional)] size: Option<TabSize>,
    #[prop(optional)] variant: Option<TabVariant>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let size = size.unwrap_or_default();
    let variant = variant.unwrap_or_default();
    let root_class = join_classes(&[
        Some(TABS_LIST_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let tabs_view: Vec<_> = tabs
        .iter()
        .map(|tab| {
            let selected = tab.value == value;
            let label = tab.label.clone();
            let value_attr = tab.value.clone();
            view! {
                <button
                    type="button"
                    class=join_classes(&[Some(TABS_TAB_CLASS), Some(TAB_BUTTON_CLASS)])
                    role="tab"
                    aria-selected=selected.then_some("true")
                    data-highlighted=selected.then_some("")
                    data-size=size.as_str()
                    data-variant=variant.as_str()
                    data-value=value_attr
                >
                    <span class=TABS_CONTENT_CLASS>
                        <span class=TABS_START_ICON_CLASS aria-hidden="true"></span>
                        <span class=TABS_LABEL_CLASS>{label}</span>
                    </span>
                </button>
            }
        })
        .collect();
    view! {
        <div class=root_class role="tablist" data-size=size.as_str() data-variant=variant.as_str()>
            {tabs_view}
            <div class=TABS_INDICATOR_CLASS aria-hidden="true"></div>
        </div>
    }
}

#[component]
pub fn TabPanel(
    #[prop(optional)] value: &'static str,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let panel_class = join_classes(&[
        Some(TABS_PANEL_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let slot = children.map(|c| c().into_any());
    view! {
        <div class=panel_class role="tabpanel" data-value=value>{slot}</div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tablist_emits_tablist_role_and_tabs() {
        let out = (view! {
            <TabList
                tabs=vec![TabItem::new("a", "Alpha"), TabItem::new("b", "Beta")]
                value="a"
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains(TABS_LIST_CLASS), "{out}");
        assert!(out.contains("role=\"tablist\""), "{out}");
        assert!(out.matches(TABS_TAB_CLASS).count() >= 2, "{out}");
        assert!(out.contains("role=\"tab\""), "{out}");
        assert!(out.contains(">Alpha<"), "{out}");
        assert!(out.contains(">Beta<"), "{out}");
        assert!(out.contains("data-highlighted"), "{out}");
        assert!(out.contains("aria-selected=\"true\""), "{out}");
        assert!(out.contains("data-size=\"md\""), "{out}");
        assert!(out.contains("data-variant=\"underlined\""), "{out}");
    }

    #[test]
    fn selected_tab_matches_value() {
        let out = (view! {
            <TabList
                tabs=vec![TabItem::new("a", "A"), TabItem::new("b", "B")]
                value="b"
            />
        })
        .into_view()
        .to_html();
        let b_idx = out.find(">B<").unwrap();
        let a_idx = out.find(">A<").unwrap();
        let b_selected = out[..b_idx].matches("data-highlighted").count();
        let a_selected = out[..a_idx].matches("data-highlighted").count();
        assert_eq!(a_selected, 0, "tab a must not be highlighted");
        assert!(b_selected > a_selected, "tab b must be highlighted");
        assert!(out.contains("data-value=\"b\""), "{out}");
    }

    #[test]
    fn indicator_and_tab_button_classes() {
        let out = (view! { <TabList tabs=vec![TabItem::new("x", "X")] size=TabSize::Sm/> })
            .into_view()
            .to_html();
        assert!(out.contains(TABS_INDICATOR_CLASS), "{out}");
        assert!(out.contains(TAB_BUTTON_CLASS), "{out}");
        assert!(out.contains(TABS_LABEL_CLASS), "{out}");
        assert!(out.contains("data-size=\"sm\""), "{out}");
    }

    #[test]
    fn tab_panel_uses_role_and_value() {
        let out = (view! { <TabPanel value="a">{"Body"}</TabPanel> })
            .into_view()
            .to_html();
        assert!(out.contains(TABS_PANEL_CLASS), "{out}");
        assert!(out.contains("role=\"tabpanel\""), "{out}");
        assert!(out.contains("data-value=\"a\""), "{out}");
        assert!(out.contains(">Body<"), "{out}");
    }
}
