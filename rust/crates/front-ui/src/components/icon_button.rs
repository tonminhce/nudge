use leptos::prelude::*;

use crate::components::button::{
    join_classes, ButtonColor, ButtonVariant, BUTTON_CLASS, BUTTON_CONTENT_CLASS,
    BUTTON_LABEL_CLASS,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum IconButtonSize {
    Xs,
    Sm,
    #[default]
    Md,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum IconButtonShape {
    #[default]
    Square,
    Round,
}

impl IconButtonSize {
    pub fn as_str(self) -> &'static str {
        match self {
            IconButtonSize::Xs => "xs",
            IconButtonSize::Sm => "sm",
            IconButtonSize::Md => "md",
        }
    }

    pub fn resolved_button_size(self) -> &'static str {
        match self {
            IconButtonSize::Xs => "sm",
            IconButtonSize::Sm => "sm",
            IconButtonSize::Md => "md",
        }
    }
}

impl IconButtonShape {
    pub fn as_str(self) -> &'static str {
        match self {
            IconButtonShape::Square => "square",
            IconButtonShape::Round => "round",
        }
    }
}

pub const ICON_BUTTON_CLASS: &str = "t-iconButton-button";
pub const ICON_BUTTON_ICON_CLASS: &str = "t-iconButton-icon";
pub const ICON_BUTTON_TOOLTIP_ANCHOR_CLASS: &str = "t-iconButton-tooltipAnchor";

#[component]
pub fn IconButton(
    children: Children,
    #[prop(optional)] size: Option<IconButtonSize>,
    #[prop(optional)] shape: Option<IconButtonShape>,
    #[prop(optional)] variant: Option<ButtonVariant>,
    #[prop(optional)] color: Option<ButtonColor>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional)] tooltip: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let size = size.unwrap_or_default();
    let shape = shape.unwrap_or_default();
    let variant = variant.unwrap_or(ButtonVariant::Ghost);
    let color = color.unwrap_or(ButtonColor::Neutral);
    let root_class = join_classes(&[
        Some(BUTTON_CLASS),
        Some(ICON_BUTTON_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let effective_disabled = disabled || loading;
    let icon = children();
    let button = view! {
        <button
            type="button"
            class=root_class
            data-variant=variant.as_str()
            data-color=color.as_str()
            data-size=size.resolved_button_size()
            data-icon-size=size.as_str()
            data-shape=shape.as_str()
            data-loading=loading.then_some("")
            disabled=effective_disabled
        >
            <span class=BUTTON_CONTENT_CLASS>
                <span class=ICON_BUTTON_ICON_CLASS aria-hidden="true">{icon}</span>
                <span class=BUTTON_LABEL_CLASS>{""}</span>
            </span>
        </button>
    };
    let has_tooltip = !tooltip.is_empty();
    view! {
        {if has_tooltip {
            view! {
                <span class=ICON_BUTTON_TOOLTIP_ANCHOR_CLASS
                    data-tooltip=tooltip
                    aria-label=tooltip
                >{button.into_any()}</span>
            }
            .into_any()
        } else {
            button.into_any()
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_button_carries_icon_size_and_shape_attributes() {
        let out = (view! {
            <IconButton size=IconButtonSize::Xs shape=IconButtonShape::Round>
                <svg />
            </IconButton>
        })
        .into_view()
        .to_html();
        assert!(out.contains(BUTTON_CLASS), "{out}");
        assert!(out.contains(ICON_BUTTON_CLASS), "{out}");
        assert!(out.contains("data-icon-size=\"xs\""), "{out}");
        assert!(out.contains("data-shape=\"round\""), "{out}");
        assert!(out.contains(ICON_BUTTON_ICON_CLASS), "{out}");
        assert!(out.contains("data-size=\"sm\""), "{out}");
        assert!(out.contains("data-variant=\"ghost\""), "{out}");
        assert!(!out.contains(ICON_BUTTON_TOOLTIP_ANCHOR_CLASS), "{out}");
    }

    #[test]
    fn tooltip_wraps_the_button_in_an_anchor() {
        let out = (view! {
            <IconButton tooltip="Close"><svg /></IconButton>
        })
        .into_view()
        .to_html();
        assert!(out.contains(ICON_BUTTON_TOOLTIP_ANCHOR_CLASS), "{out}");
        assert!(out.contains("data-tooltip=\"Close\""), "{out}");
        assert!(out.contains("aria-label=\"Close\""), "{out}");
    }

    #[test]
    fn size_mapping_collapses_xs_to_sm_button() {
        assert_eq!(IconButtonSize::Xs.resolved_button_size(), "sm");
        assert_eq!(IconButtonSize::Sm.resolved_button_size(), "sm");
        assert_eq!(IconButtonSize::Md.resolved_button_size(), "md");
        assert_eq!(IconButtonSize::Md.as_str(), "md");
    }

    #[test]
    fn loading_disables_like_upstream() {
        let out = (view! { <IconButton loading=true><svg/></IconButton> })
            .into_view()
            .to_html();
        assert!(out.contains("data-loading"), "{out}");
        assert!(out.contains("disabled"), "{out}");
    }
}
