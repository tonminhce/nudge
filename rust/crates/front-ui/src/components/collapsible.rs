use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;
use crate::components::next_dom_id;

pub const COLLAPSIBLE_CLASS: &str = "t-collapsible";
pub const COLLAPSIBLE_TRIGGER_CLASS: &str = "t-collapsible-trigger";
pub const COLLAPSIBLE_CONTENT_CLASS: &str = "t-collapsible-content";

#[component]
pub fn Collapsible(
    trigger: AnyView,
    content: AnyView,
    #[prop(optional, into)] open: Signal<bool>,
    #[prop(optional, into)] on_open_change: Option<Callback<bool>>,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] id: &'static str,
) -> impl IntoView {
    let root_class = join_classes(&[
        Some(COLLAPSIBLE_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let base_id = if id.is_empty() {
        next_dom_id("t-collapsible")
    } else {
        id.to_string()
    };
    let trigger_id = format!("{base_id}-trigger");
    let content_id = format!("{base_id}-content");
    view! {
        <div
            class=root_class
            data-state=move || if open.get() { "open" } else { "closed" }
        >
            <button
                id=trigger_id.clone()
                type="button"
                class=COLLAPSIBLE_TRIGGER_CLASS
                aria-expanded=move || open.get().to_string()
                aria-controls=content_id.clone()
                data-state=move || if open.get() { "open" } else { "closed" }
                on:click=move |_| {
                    if let Some(on_open_change) = on_open_change {
                        on_open_change.run(!open.get());
                    }
                }
            >
                {trigger}
            </button>
            <div
                id=content_id
                class=COLLAPSIBLE_CONTENT_CLASS
                role="region"
                aria-labelledby=trigger_id
                data-state=move || if open.get() { "open" } else { "closed" }
                hidden=move || !open.get()
            >
                {content}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_collapsible_links_trigger_and_region() {
        let out = (view! {
            <Collapsible
                open=true
                id="details-collapsible"
                class="details"
                trigger=view! { "Details" }.into_any()
                content=view! { "More information" }.into_any()
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-collapsible details\""), "{out}");
        assert!(out.contains("data-state=\"open\""), "{out}");
        assert!(out.contains("type=\"button\""), "{out}");
        assert!(out.contains("aria-expanded=\"true\""), "{out}");
        assert!(
            out.contains("aria-controls=\"details-collapsible-content\""),
            "{out}"
        );
        assert!(out.contains("id=\"details-collapsible-trigger\""), "{out}");
        assert!(out.contains("role=\"region\""), "{out}");
        assert!(
            out.contains("aria-labelledby=\"details-collapsible-trigger\""),
            "{out}"
        );
        assert!(out.contains(">Details<"), "{out}");
        assert!(out.contains(">More information<"), "{out}");
        assert!(!out.contains("hidden"), "{out}");
    }

    #[test]
    fn closed_collapsible_hides_content() {
        let out = (view! {
            <Collapsible
                trigger=view! { "Show" }.into_any()
                content=view! { "Hidden body" }.into_any()
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-collapsible\""), "{out}");
        assert!(out.contains("data-state=\"closed\""), "{out}");
        assert!(out.contains("aria-expanded=\"false\""), "{out}");
        assert!(out.contains("hidden"), "{out}");
        assert!(out.contains(">Hidden body<"), "{out}");
    }

    #[test]
    fn signal_and_open_change_callback_compile_through_props_builder() {
        let (open, set_open) = signal(false);
        let props = CollapsibleProps::builder()
            .open(open)
            .on_open_change(move |next: bool| set_open.set(next))
            .trigger(view! { "Show" }.into_any())
            .content(view! { "Body" }.into_any())
            .build();
        let out = Collapsible(props).into_view().to_html();
        assert!(out.contains("aria-expanded=\"false\""), "{out}");
        assert!(out.contains("hidden"), "{out}");

        set_open.set(true);
        let out = Collapsible(
            CollapsibleProps::builder()
                .open(open)
                .trigger(view! { "Show" }.into_any())
                .content(view! { "Body" }.into_any())
                .build(),
        )
        .into_view()
        .to_html();
        assert!(out.contains("aria-expanded=\"true\""), "{out}");
        assert!(!out.contains("hidden"), "{out}");
    }
}
