use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TooltipSide {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

impl TooltipSide {
    pub fn as_str(self) -> &'static str {
        match self {
            TooltipSide::Top => "top",
            TooltipSide::Right => "right",
            TooltipSide::Bottom => "bottom",
            TooltipSide::Left => "left",
        }
    }
}

pub const TOOLTIP_POSITIONER_CLASS: &str = "t-tooltip-positioner";
pub const TOOLTIP_POPUP_CLASS: &str = "t-tooltip-popup";
pub const TOOLTIP_CONTENT_CLASS: &str = "t-tooltip-content";
pub const TOOLTIP_TITLE_CLASS: &str = "t-tooltip-title";
pub const TOOLTIP_ICON_CLASS: &str = "t-tooltip-icon";
pub const TOOLTIP_DESCRIPTION_CLASS: &str = "t-tooltip-description";
pub const TOOLTIP_ARROW_CLASS: &str = "t-tooltip-arrow";

#[component]
pub fn Tooltip(
    content: &'static str,
    #[prop(optional)] title: &'static str,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] icon: Option<AnyView>,
    #[prop(optional)] side: Option<TooltipSide>,
    #[prop(optional)] open: bool,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let side = side.unwrap_or_default();
    let root_class = join_classes(&[
        Some(TOOLTIP_POPUP_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let title_present = !title.is_empty();
    let description_present = !description.is_empty();
    let icon_node = icon;
    view! {
        <div class=TOOLTIP_POSITIONER_CLASS data-side=side.as_str()>
            <div
                class=root_class
                role="tooltip"
                data-open=open.then_some("")
                style="background: var(--t-tooltip-background); color: var(--t-tooltip-color);"
            >
                <div class=TOOLTIP_CONTENT_CLASS>
                    {icon_node.map(|node| view! { <span class=TOOLTIP_ICON_CLASS>{node}</span> })}
                    {title_present.then(|| view! { <span class=TOOLTIP_TITLE_CLASS>{title}</span> })}
                    {description_present.then(|| {
                        view! { <span class=TOOLTIP_DESCRIPTION_CLASS>{description}</span> }
                    })}
                    {if !title_present && !description_present {
                        Some(view! { <span class=TOOLTIP_TITLE_CLASS>{content}</span> })
                    } else {
                        None
                    }}
                </div>
                <div class=TOOLTIP_ARROW_CLASS></div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_uses_semantic_background_tokens() {
        let out = (view! { <Tooltip content="Save" /> }).into_view().to_html();
        assert!(out.contains(TOOLTIP_POSITIONER_CLASS), "{out}");
        assert!(out.contains(TOOLTIP_POPUP_CLASS), "{out}");
        assert!(out.contains("role=\"tooltip\""), "{out}");
        assert!(
            out.contains("background: var(--t-tooltip-background)"),
            "{out}"
        );
        assert!(out.contains("color: var(--t-tooltip-color)"), "{out}");
        assert!(out.contains(TOOLTIP_TITLE_CLASS), "{out}");
        assert!(out.contains(">Save<"), "{out}");
        assert!(out.contains(TOOLTIP_ARROW_CLASS), "{out}");
        assert!(out.contains("data-side=\"right\""), "{out}");
    }

    #[test]
    fn tooltip_title_and_description_are_slots() {
        let out = (view! {
            <Tooltip
                content="fallback"
                title="Heading"
                description="More detail"
                side=TooltipSide::Top
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains(">Heading<"), "{out}");
        assert!(out.contains(TOOLTIP_DESCRIPTION_CLASS), "{out}");
        assert!(out.contains(">More detail<"), "{out}");
        assert!(!out.contains(">fallback<"), "{out}");
        assert!(out.contains("data-side=\"top\""), "{out}");
    }

    #[test]
    fn open_state_marks_data_open() {
        let out = (view! { <Tooltip content="x" open=true /> })
            .into_view()
            .to_html();
        assert!(out.contains("data-open"), "{out}");
        let out = (view! { <Tooltip content="x" /> }).into_view().to_html();
        assert!(!out.contains("data-open"), "{out}");
    }
}
