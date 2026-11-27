use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;
use crate::components::next_dom_id;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PopoverPlacement {
    TopStart,
    Top,
    TopEnd,
    RightStart,
    Right,
    RightEnd,
    #[default]
    BottomStart,
    Bottom,
    BottomEnd,
    LeftStart,
    Left,
    LeftEnd,
}

impl PopoverPlacement {
    pub fn as_str(self) -> &'static str {
        match self {
            PopoverPlacement::TopStart => "top-start",
            PopoverPlacement::Top => "top",
            PopoverPlacement::TopEnd => "top-end",
            PopoverPlacement::RightStart => "right-start",
            PopoverPlacement::Right => "right",
            PopoverPlacement::RightEnd => "right-end",
            PopoverPlacement::BottomStart => "bottom-start",
            PopoverPlacement::Bottom => "bottom",
            PopoverPlacement::BottomEnd => "bottom-end",
            PopoverPlacement::LeftStart => "left-start",
            PopoverPlacement::Left => "left",
            PopoverPlacement::LeftEnd => "left-end",
        }
    }
}

pub const POPOVER_CLASS: &str = "t-popover";
pub const POPOVER_TRIGGER_CLASS: &str = "t-popover-trigger";
pub const POPOVER_CONTENT_CLASS: &str = "t-popover-content";

#[component]
pub fn Popover(
    trigger: AnyView,
    content: AnyView,
    #[prop(optional, into)] open: Signal<bool>,
    #[prop(optional, into)] on_open_change: Option<Callback<bool>>,
    #[prop(optional)] placement: Option<PopoverPlacement>,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] id: &'static str,
) -> impl IntoView {
    let placement = placement.unwrap_or_default();
    let root_class = join_classes(&[
        Some(POPOVER_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let base_id = if id.is_empty() {
        next_dom_id("t-popover")
    } else {
        id.to_string()
    };
    let trigger_id = format!("{base_id}-trigger");
    let content_id = format!("{base_id}-content");
    view! {
        <div
            class=root_class
            data-state=move || if open.get() { "open" } else { "closed" }
            data-placement=placement.as_str()
        >
            <button
                id=trigger_id.clone()
                type="button"
                class=POPOVER_TRIGGER_CLASS
                aria-haspopup="dialog"
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
                class=POPOVER_CONTENT_CLASS
                role="dialog"
                aria-labelledby=trigger_id
                data-state=move || if open.get() { "open" } else { "closed" }
                data-placement=placement.as_str()
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
    fn open_popover_links_trigger_and_dialog() {
        let out = (view! {
            <Popover
                open=true
                id="profile-popover"
                placement=PopoverPlacement::TopEnd
                class="profile-popover"
                trigger=view! { "Profile" }.into_any()
                content=view! { "Account details" }.into_any()
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-popover profile-popover\""), "{out}");
        assert!(out.contains("data-placement=\"top-end\""), "{out}");
        assert!(out.contains("data-state=\"open\""), "{out}");
        assert!(out.contains("type=\"button\""), "{out}");
        assert!(out.contains("aria-haspopup=\"dialog\""), "{out}");
        assert!(out.contains("aria-expanded=\"true\""), "{out}");
        assert!(
            out.contains("aria-controls=\"profile-popover-content\""),
            "{out}"
        );
        assert!(out.contains("id=\"profile-popover-trigger\""), "{out}");
        assert!(out.contains("role=\"dialog\""), "{out}");
        assert!(
            out.contains("aria-labelledby=\"profile-popover-trigger\""),
            "{out}"
        );
        assert!(out.contains(">Profile<"), "{out}");
        assert!(out.contains(">Account details<"), "{out}");
        assert!(!out.contains("hidden"), "{out}");
    }

    #[test]
    fn closed_popover_hides_content_and_uses_default_placement() {
        let out = (view! {
            <Popover
                trigger=view! { "Open" }.into_any()
                content=view! { "Details" }.into_any()
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-popover\""), "{out}");
        assert!(out.contains("data-state=\"closed\""), "{out}");
        assert!(out.contains("data-placement=\"bottom-start\""), "{out}");
        assert!(out.contains("aria-expanded=\"false\""), "{out}");
        assert!(out.contains("hidden"), "{out}");
    }

    #[test]
    fn signal_and_open_change_callback_compile_through_props_builder() {
        let (open, set_open) = signal(false);
        let props = PopoverProps::builder()
            .open(open)
            .on_open_change(move |next: bool| set_open.set(next))
            .trigger(view! { "Open" }.into_any())
            .content(view! { "Details" }.into_any())
            .build();
        let out = Popover(props).into_view().to_html();
        assert!(out.contains("aria-expanded=\"false\""), "{out}");

        set_open.set(true);
        let out = Popover(
            PopoverProps::builder()
                .open(open)
                .trigger(view! { "Open" }.into_any())
                .content(view! { "Details" }.into_any())
                .build(),
        )
        .into_view()
        .to_html();
        assert!(out.contains("aria-expanded=\"true\""), "{out}");
    }

    #[test]
    fn placement_strings_cover_all_sides_and_alignments() {
        for (placement, expected) in [
            (PopoverPlacement::TopStart, "top-start"),
            (PopoverPlacement::Right, "right"),
            (PopoverPlacement::BottomEnd, "bottom-end"),
            (PopoverPlacement::LeftStart, "left-start"),
        ] {
            assert_eq!(placement.as_str(), expected);
        }
    }
}
