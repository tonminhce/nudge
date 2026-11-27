use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BreadcrumbItem {
    pub label: String,
    pub href: String,
}

impl BreadcrumbItem {
    pub fn new(label: &str, href: &str) -> Self {
        Self {
            label: label.to_string(),
            href: href.to_string(),
        }
    }

    pub fn current(label: &str) -> Self {
        Self {
            label: label.to_string(),
            href: String::new(),
        }
    }
}

pub const BREADCRUMB_CLASS: &str = "t-breadcrumb";
pub const BREADCRUMB_LIST_CLASS: &str = "t-breadcrumb-list";
pub const BREADCRUMB_ITEM_CLASS: &str = "t-breadcrumb-item";
pub const BREADCRUMB_LINK_CLASS: &str = "t-breadcrumb-link";
pub const BREADCRUMB_CURRENT_CLASS: &str = "t-breadcrumb-current";
pub const BREADCRUMB_SEPARATOR_CLASS: &str = "t-breadcrumb-separator";

#[component]
pub fn Breadcrumb(
    items: Vec<BreadcrumbItem>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let root_class = join_classes(&[
        Some(BREADCRUMB_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let item_count = items.len();
    let items_view: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let current = index + 1 == item_count;
            let label = item.label.clone();
            let href = item.href.clone();
            let link_class = join_classes(&[
                Some(BREADCRUMB_LINK_CLASS),
                current.then_some(BREADCRUMB_CURRENT_CLASS),
            ]);
            view! {
                <li
                    class=BREADCRUMB_ITEM_CLASS
                    data-current=current.then_some("")
                >
                    {if href.is_empty() {
                        view! {
                            <span
                                class=link_class
                                aria-current=current.then_some("page")
                            >{label}</span>
                        }
                        .into_any()
                    } else {
                        view! {
                            <a
                                class=link_class
                                href=href
                                aria-current=current.then_some("page")
                            >{label}</a>
                        }
                        .into_any()
                    }}
                    {if current {
                        None
                    } else {
                        Some(view! {
                            <span class=BREADCRUMB_SEPARATOR_CLASS aria-hidden="true">{"/"}</span>
                        })
                    }}
                </li>
            }
        })
        .collect();
    view! {
        <nav class=root_class aria-label="Breadcrumb">
            <ol class=BREADCRUMB_LIST_CLASS>{items_view}</ol>
        </nav>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breadcrumb_uses_nav_and_ordered_list_with_current_page() {
        let out = (view! {
            <Breadcrumb
                items=vec![
                    BreadcrumbItem::new("Home", "/"),
                    BreadcrumbItem::new("Projects", "/projects"),
                    BreadcrumbItem::current("Nudge"),
                ]
                class="page-breadcrumb"
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("<nav"), "{out}");
        assert!(out.contains("aria-label=\"Breadcrumb\""), "{out}");
        assert!(
            out.contains("class=\"t-breadcrumb page-breadcrumb\""),
            "{out}"
        );
        assert!(out.contains("<ol"), "{out}");
        assert_eq!(out.matches("<li").count(), 3, "{out}");
        assert_eq!(out.matches("aria-current=\"page\"").count(), 1, "{out}");
        assert!(out.contains("data-current"), "{out}");
        assert!(out.contains(BREADCRUMB_CURRENT_CLASS), "{out}");
        assert!(out.contains("href=\"/\""), "{out}");
        assert!(out.contains("href=\"/projects\""), "{out}");
        assert!(out.contains(">Nudge<"), "{out}");
        assert_eq!(out.matches(BREADCRUMB_SEPARATOR_CLASS).count(), 2, "{out}");
        assert!(out.contains("aria-hidden=\"true\""), "{out}");
    }

    #[test]
    fn linked_last_item_is_still_marked_as_current() {
        let out = (view! {
            <Breadcrumb items=vec![BreadcrumbItem::new("Settings", "/settings")]/>
        })
        .into_view()
        .to_html();
        assert!(out.contains("href=\"/settings\""), "{out}");
        assert!(out.contains("aria-current=\"page\""), "{out}");
        assert!(!out.contains(BREADCRUMB_SEPARATOR_CLASS), "{out}");
    }

    #[test]
    fn empty_breadcrumb_and_item_constructors_are_well_formed() {
        let out = (view! { <Breadcrumb items=vec![]/> }).into_view().to_html();
        assert!(out.contains("<ol"), "{out}");
        assert!(!out.contains("<li"), "{out}");
        assert!(!out.contains("aria-current"), "{out}");
        assert_eq!(
            BreadcrumbItem::current("Here"),
            BreadcrumbItem {
                label: "Here".into(),
                href: String::new(),
            }
        );
    }
}
