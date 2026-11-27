use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BadgeTone {
    #[default]
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

impl BadgeTone {
    pub fn as_str(self) -> &'static str {
        match self {
            BadgeTone::Neutral => "neutral",
            BadgeTone::Accent => "accent",
            BadgeTone::Success => "success",
            BadgeTone::Warning => "warning",
            BadgeTone::Danger => "danger",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BadgeVariant {
    Solid,
    Outline,
    #[default]
    Soft,
}

impl BadgeVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            BadgeVariant::Solid => "solid",
            BadgeVariant::Outline => "outline",
            BadgeVariant::Soft => "soft",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BadgeSize {
    Sm,
    #[default]
    Md,
}

impl BadgeSize {
    pub fn as_str(self) -> &'static str {
        match self {
            BadgeSize::Sm => "sm",
            BadgeSize::Md => "md",
        }
    }
}

pub const BADGE_CLASS: &str = "t-badge";

#[component]
pub fn Badge(
    #[prop(optional)] label: &'static str,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] tone: Option<BadgeTone>,
    #[prop(optional)] variant: Option<BadgeVariant>,
    #[prop(optional)] size: Option<BadgeSize>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let tone = tone.unwrap_or_default();
    let variant = variant.unwrap_or_default();
    let size = size.unwrap_or_default();
    let root_class = join_classes(&[
        Some(BADGE_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let content = children
        .map(|children| children().into_any())
        .unwrap_or_else(|| view! { {label} }.into_any());
    view! {
        <span
            class=root_class
            data-tone=tone.as_str()
            data-variant=variant.as_str()
            data-size=size.as_str()
        >
            {content}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_label_renders_tone_variant_size_and_custom_class() {
        let out = (view! {
            <Badge
                label="New"
                tone=BadgeTone::Success
                variant=BadgeVariant::Solid
                size=BadgeSize::Sm
                class="status"
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-badge status\""), "{out}");
        assert!(out.contains("data-tone=\"success\""), "{out}");
        assert!(out.contains("data-variant=\"solid\""), "{out}");
        assert!(out.contains("data-size=\"sm\""), "{out}");
        assert!(out.contains(">New<"), "{out}");
    }

    #[test]
    fn badge_accepts_children_and_defaults_to_neutral_soft_md() {
        let out = (view! { <Badge><strong>{"Child"}</strong></Badge> })
            .into_view()
            .to_html();
        assert!(out.contains("data-tone=\"neutral\""), "{out}");
        assert!(out.contains("data-variant=\"soft\""), "{out}");
        assert!(out.contains("data-size=\"md\""), "{out}");
        assert!(out.contains("<strong>Child</strong>"), "{out}");
    }

    #[test]
    fn badge_props_builder_accepts_label_and_children() {
        let props = BadgeProps::builder()
            .label("New")
            .tone(BadgeTone::Accent)
            .variant(BadgeVariant::Outline)
            .size(BadgeSize::Sm)
            .build();
        let out = Badge(props).into_view().to_html();
        assert!(out.contains(">New<"), "{out}");
        assert!(out.contains("data-tone=\"accent\""), "{out}");

        let props = BadgeProps::builder()
            .children(ToChildren::to_children(|| "Child"))
            .build();
        let out = Badge(props).into_view().to_html();
        assert!(out.contains(">Child<"), "{out}");
    }
}
