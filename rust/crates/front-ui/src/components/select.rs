use leptos::prelude::*;

use crate::components::button::join_classes;
use crate::components::input::{InputSize, INPUT_CLASS};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SelectItem {
    pub label: String,
    pub selected: bool,
    pub disabled: bool,
}

impl SelectItem {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            selected: false,
            disabled: false,
        }
    }

    pub fn selected(label: &str) -> Self {
        Self {
            label: label.to_string(),
            selected: true,
            disabled: false,
        }
    }
}

pub const SELECT_TRIGGER_CLASS: &str = "t-select-trigger";
pub const SELECT_VALUE_CLASS: &str = "t-select-value";
pub const SELECT_ICON_CLASS: &str = "t-select-icon";
pub const SELECT_POSITIONER_CLASS: &str = "t-select-positioner";
pub const SELECT_POPUP_CLASS: &str = "t-select-popup";
pub const SELECT_ITEM_CLASS: &str = "t-select-item";
pub const SELECT_INDICATOR_CLASS: &str = "t-select-indicator";
pub const SELECT_GROUP_LABEL_CLASS: &str = "t-select-groupLabel";
pub const SELECT_SEPARATOR_CLASS: &str = "t-select-separator";

pub fn select_trigger_class(size: InputSize, extra: &str) -> String {
    join_classes(&[
        Some(INPUT_CLASS),
        Some(size.class()),
        Some(SELECT_TRIGGER_CLASS),
        if extra.is_empty() { None } else { Some(extra) },
    ])
}

#[component]
pub fn Select(
    placeholder: &'static str,
    #[prop(optional)] value: &'static str,
    #[prop(optional)] open: bool,
    #[prop(optional)] size: Option<InputSize>,
    #[prop(optional)] items: Vec<SelectItem>,
    #[prop(optional)] group_label: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let size = size.unwrap_or_default();
    let trigger_class = select_trigger_class(size, class);
    let items_view: Vec<_> = items
        .iter()
        .map(|item| {
            let label = item.label.clone();
            let selected = item.selected;
            let disabled = item.disabled;
            view! {
                <div
                    class=SELECT_ITEM_CLASS
                    role="option"
                    aria-selected=selected.then_some("true")
                    data-selected=selected.then_some("")
                    data-disabled=disabled.then_some("")
                >
                    <span class=SELECT_INDICATOR_CLASS></span>
                    {label}
                </div>
            }
        })
        .collect();
    let shown = if value.is_empty() { placeholder } else { value }.to_string();
    let group = if group_label.is_empty() {
        None
    } else {
        Some(group_label.to_string())
    };
    view! {
        <div class=SELECT_POSITIONER_CLASS data-open=open.then_some("")>
            <button
                type="button"
                class=trigger_class
                data-size=size.as_str()
                aria-haspopup="listbox"
                aria-expanded=open.then_some("true")
            >
                <span class=SELECT_VALUE_CLASS>{shown}</span>
                <span class=SELECT_ICON_CLASS aria-hidden="true"></span>
            </button>
            <div class=SELECT_POPUP_CLASS role="listbox">
                {group.map(|label| view! { <div class=SELECT_GROUP_LABEL_CLASS>{label}</div> })}
                <div class=SELECT_SEPARATOR_CLASS></div>
                {items_view}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_uses_input_group_classes_and_select_classes() {
        let out = (view! {
            <Select
                placeholder="Pick"
                items=vec![SelectItem { label: "A".into(), selected: true, disabled: false }]
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains(SELECT_TRIGGER_CLASS), "{out}");
        assert!(out.contains(INPUT_CLASS), "{out}");
        assert!(out.contains("t-input-md"), "{out}");
        assert!(out.contains(">Pick<"), "{out}");
        assert!(out.contains(SELECT_VALUE_CLASS), "{out}");
        assert!(out.contains(SELECT_ICON_CLASS), "{out}");
        assert!(out.contains(SELECT_POPUP_CLASS), "{out}");
        assert!(out.contains("role=\"listbox\""), "{out}");
        assert!(out.contains(SELECT_ITEM_CLASS), "{out}");
        assert!(out.contains("data-selected"), "{out}");
        assert!(out.contains(SELECT_INDICATOR_CLASS), "{out}");
        assert!(out.contains("aria-haspopup=\"listbox\""), "{out}");
    }

    #[test]
    fn selected_value_replaces_placeholder() {
        let out = (view! { <Select placeholder="Pick" value="Chosen"/> })
            .into_view()
            .to_html();
        assert!(out.contains(">Chosen<"), "{out}");
        assert!(!out.contains(">Pick<"), "{out}");
    }

    #[test]
    fn select_separator_and_group_label_render() {
        let out = (view! { <Select placeholder="p" group_label="Group"/> })
            .into_view()
            .to_html();
        assert!(out.contains(SELECT_GROUP_LABEL_CLASS), "{out}");
        assert!(out.contains(">Group<"), "{out}");
        assert!(out.contains(SELECT_SEPARATOR_CLASS), "{out}");
    }

    #[test]
    fn select_size_maps_to_input_size_classes() {
        let out = (view! { <Select placeholder="p" size=InputSize::Sm/> })
            .into_view()
            .to_html();
        assert!(out.contains("t-input-sm"), "{out}");
        assert!(out.contains("data-size=\"sm\""), "{out}");
    }

    #[test]
    fn trigger_class_composes_input_and_select_contracts() {
        let cls = select_trigger_class(InputSize::Md, "");
        assert_eq!(
            cls,
            format!("{INPUT_CLASS} t-input-md {SELECT_TRIGGER_CLASS}"),
            "trigger class must equal the upstream composition (SelectTrigger.tsx:18)"
        );
    }

    #[test]
    fn open_state_marks_positioner_and_expanded() {
        let out = (view! { <Select placeholder="p" open=true/> })
            .into_view()
            .to_html();
        assert!(out.contains("data-open"), "{out}");
        assert!(out.contains("aria-expanded=\"true\""), "{out}");
        let out = (view! { <Select placeholder="p"/> }).into_view().to_html();
        assert!(!out.contains("data-open"), "{out}");
    }
}
