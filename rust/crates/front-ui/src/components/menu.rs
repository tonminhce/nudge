use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuPlacement {
    #[default]
    BottomStart,
    Bottom,
    BottomEnd,
    TopStart,
    Top,
    TopEnd,
}

impl MenuPlacement {
    pub fn as_str(self) -> &'static str {
        match self {
            MenuPlacement::BottomStart => "bottom-start",
            MenuPlacement::Bottom => "bottom",
            MenuPlacement::BottomEnd => "bottom-end",
            MenuPlacement::TopStart => "top-start",
            MenuPlacement::Top => "top",
            MenuPlacement::TopEnd => "top-end",
        }
    }
}

pub const MENU_ROOT_CLASS: &str = "t-menu-root";
pub const MENU_CONTENT_CLASS: &str = "t-menu-content";
pub const MENU_SECTION_CLASS: &str = "t-menu-section";
pub const MENU_GROUP_CLASS: &str = "t-menu-group";
pub const MENU_GROUP_LABEL_CLASS: &str = "t-menu-groupLabel";
pub const MENU_SEPARATOR_CLASS: &str = "t-menu-separator";
pub const MENU_ITEM_CLASS: &str = "t-menu-item";
pub const MENU_ITEM_LABEL_CLASS: &str = "t-menu-itemLabel";
pub const MENU_ITEM_START_ICON_CLASS: &str = "t-menu-itemStartIcon";
pub const MENU_ITEM_END_CLASS: &str = "t-menu-itemEnd";

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MenuItemDef {
    pub label: String,
    pub selected: bool,
    pub disabled: bool,
    pub accent: &'static str,
}

impl MenuItemDef {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            selected: false,
            disabled: false,
            accent: "default",
        }
    }

    pub fn selected(label: &str) -> Self {
        Self {
            selected: true,
            ..Self::new(label)
        }
    }
}

#[component]
pub fn Menu(
    #[prop(optional)] items: Vec<MenuItemDef>,
    #[prop(optional)] open: bool,
    #[prop(optional)] placement: Option<MenuPlacement>,
    #[prop(optional)] group_label: &'static str,
    #[prop(optional)] header: Option<AnyView>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let placement = placement.unwrap_or_default();
    let root_class = join_classes(&[
        Some(MENU_ROOT_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let items_view: Vec<_> = items
        .iter()
        .map(|item| {
            let label = item.label.clone();
            let selected = item.selected;
            let disabled = item.disabled;
            let accent = item.accent;
            view! {
                <div
                    class=MENU_ITEM_CLASS
                    role="menuitem"
                    aria-disabled=disabled.then_some("true")
                    data-accent=accent
                    data-selected=selected.then_some("")
                    data-disabled=disabled.then_some("")
                    tabindex="-1"
                >
                    <span class=MENU_ITEM_START_ICON_CLASS aria-hidden="true"></span>
                    <span class=MENU_ITEM_LABEL_CLASS>{label}</span>
                    <span class=MENU_ITEM_END_CLASS></span>
                </div>
            }
        })
        .collect();
    let header_slot = header;
    let group = if group_label.is_empty() {
        None
    } else {
        Some(group_label.to_string())
    };
    view! {
        <div
            class=root_class
            role="menu"
            data-placement=placement.as_str()
            data-open=open.then_some("")
        >
            {header_slot.map(|node| view! { <div class=MENU_SECTION_CLASS>{node}</div> })}
            {group.map(|label| view! {
                <div class=MENU_GROUP_CLASS>
                    <div class=MENU_GROUP_LABEL_CLASS>{label}</div>
                </div>
            })}
            <div class=MENU_CONTENT_CLASS>{items_view}</div>
            <div class=MENU_SEPARATOR_CLASS role="separator"></div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_emits_menu_roles_and_item_contract() {
        let out = (view! {
            <Menu
                items=vec![
                    MenuItemDef::selected("Edit"),
                    MenuItemDef { label: "Delete".into(), selected: false, disabled: true, accent: "danger" },
                ]
                open=true
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains(MENU_ROOT_CLASS), "{out}");
        assert!(out.contains("role=\"menu\""), "{out}");
        assert!(out.contains("data-placement=\"bottom-start\""), "{out}");
        assert!(out.contains("data-open"), "{out}");
        assert!(out.contains(MENU_CONTENT_CLASS), "{out}");
        assert!(out.contains("role=\"menuitem\""), "{out}");
        assert!(out.matches("class=\"t-menu-item\"").count() == 2, "{out}");
        assert!(out.contains(">Edit<"), "{out}");
        assert!(out.contains(">Delete<"), "{out}");
        assert!(out.contains("data-selected"), "{out}");
        assert!(out.contains("aria-disabled=\"true\""), "{out}");
        assert!(out.contains("data-accent=\"danger\""), "{out}");
        assert!(out.contains(MENU_SEPARATOR_CLASS), "{out}");
    }

    #[test]
    fn menu_group_label_and_header() {
        let out = (view! {
            <Menu
                group_label="Actions"
                header=view! { <div class="h">{"Header"}</div> }.into_any()
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains(MENU_GROUP_LABEL_CLASS), "{out}");
        assert!(out.contains(">Actions<"), "{out}");
        assert!(out.contains(">Header<"), "{out}");
        assert!(out.contains(MENU_SECTION_CLASS), "{out}");
    }

    #[test]
    fn placement_tokens() {
        assert_eq!(MenuPlacement::BottomStart.as_str(), "bottom-start");
        assert_eq!(MenuPlacement::TopEnd.as_str(), "top-end");
    }
}
