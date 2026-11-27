use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;

pub const LIST_ITEM_CLASS: &str = "t-list-item";
pub const LIST_ITEM_CELL_CLASS: &str = "t-list-item-cell";
pub const LIST_ITEM_ACTION_CLASS: &str = "t-list-item-action";
pub const LIST_ITEM_TITLE_CLASS: &str = "t-list-item-title";
pub const LIST_ITEM_DESCRIPTION_CLASS: &str = "t-list-item-description";

#[component]
pub fn ListItem(
    title: &'static str,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] href: Option<&'static str>,
    #[prop(optional)] selected: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional, into)] on_click: Option<Callback<leptos::web_sys::MouseEvent>>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let root_class = join_classes(&[
        Some(LIST_ITEM_CLASS),
        selected.then_some("t-list-item-selected"),
        disabled.then_some("t-list-item-disabled"),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let contents = view! {
        <span class=LIST_ITEM_TITLE_CLASS>{title}</span>
        {if description.is_empty() {
            None
        } else {
            Some(view! { <span class=LIST_ITEM_DESCRIPTION_CLASS>{description}</span> })
        }}
    };
    let action = if disabled {
        view! {
            <span class=LIST_ITEM_ACTION_CLASS aria-disabled="true">{contents}</span>
        }
        .into_any()
    } else if let Some(href) = href {
        view! {
            <a
                class=LIST_ITEM_ACTION_CLASS
                href=href
                on:click=move |event| {
                    if let Some(on_click) = on_click {
                        on_click.run(event);
                    }
                }
            >
                {contents}
            </a>
        }
        .into_any()
    } else {
        view! {
            <button
                type="button"
                class=LIST_ITEM_ACTION_CLASS
                disabled=disabled
                on:click=move |event| {
                    if let Some(on_click) = on_click {
                        on_click.run(event);
                    }
                }
            >
                {contents}
            </button>
        }
        .into_any()
    };
    view! {
        <div
            class=root_class
            role="row"
            aria-selected=selected.to_string()
            aria-disabled=disabled.to_string()
            data-selected=selected.then_some("")
            data-disabled=disabled.then_some("")
        >
            <div class=LIST_ITEM_CELL_CLASS role="cell">{action}</div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_disabled_row_has_row_and_cell_semantics() {
        let out = (view! {
            <ListItem
                title="Archive"
                description="Move to the archive"
                href="/archive"
                selected=true
                disabled=true
                class="archive-row"
            />
        })
        .into_view()
        .to_html();
        assert!(
            out.contains(
                "class=\"t-list-item t-list-item-selected t-list-item-disabled archive-row\""
            ),
            "{out}"
        );
        assert!(out.contains("role=\"row\""), "{out}");
        assert!(out.contains("role=\"cell\""), "{out}");
        assert!(out.contains("aria-selected=\"true\""), "{out}");
        assert!(out.contains("aria-disabled=\"true\""), "{out}");
        assert!(out.contains("data-selected"), "{out}");
        assert!(out.contains("data-disabled"), "{out}");
        assert!(out.contains(LIST_ITEM_TITLE_CLASS), "{out}");
        assert!(out.contains(">Archive<"), "{out}");
        assert!(out.contains(LIST_ITEM_DESCRIPTION_CLASS), "{out}");
        assert!(out.contains(">Move to the archive<"), "{out}");
        assert!(!out.contains("href="), "{out}");
        assert!(!out.contains("<button"), "{out}");
    }

    #[test]
    fn enabled_href_renders_a_link_with_default_row_state() {
        let out = (view! {
            <ListItem title="Open record" href="/records/1"/>
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-list-item\""), "{out}");
        assert!(out.contains("href=\"/records/1\""), "{out}");
        assert!(out.contains("aria-selected=\"false\""), "{out}");
        assert!(out.contains("aria-disabled=\"false\""), "{out}");
        assert!(!out.contains("data-selected"), "{out}");
        assert!(!out.contains("data-disabled"), "{out}");
        assert!(!out.contains(LIST_ITEM_DESCRIPTION_CLASS), "{out}");
    }

    #[test]
    fn row_without_href_uses_a_button_and_accepts_click_callback() {
        let props = ListItemProps::builder()
            .title("Create")
            .on_click(|_: leptos::web_sys::MouseEvent| {})
            .build();
        let out = ListItem(props).into_view().to_html();
        assert!(out.contains("<button"), "{out}");
        assert!(out.contains("type=\"button\""), "{out}");
        assert!(out.contains(LIST_ITEM_ACTION_CLASS), "{out}");
        assert!(out.contains(">Create<"), "{out}");
        assert!(!out.contains(" disabled"), "{out}");
    }
}
