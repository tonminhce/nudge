use leptos::prelude::*;

use crate::components::button::join_classes;
use crate::tokens::HUES;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TagVariant {
    #[default]
    Soft,
    Outline,
    Ghost,
    Solid,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TagWeight {
    #[default]
    Regular,
    Medium,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TagBorderStyle {
    Solid,
    Dashed,
    Dotted,
}

impl TagVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            TagVariant::Soft => "soft",
            TagVariant::Outline => "outline",
            TagVariant::Ghost => "ghost",
            TagVariant::Solid => "solid",
        }
    }
}

impl TagWeight {
    pub fn as_str(self) -> &'static str {
        match self {
            TagWeight::Regular => "regular",
            TagWeight::Medium => "medium",
        }
    }
}

impl TagBorderStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            TagBorderStyle::Solid => "solid",
            TagBorderStyle::Dashed => "dashed",
            TagBorderStyle::Dotted => "dotted",
        }
    }
}

pub const TAG_CLASS: &str = "t-tag";
pub const TAG_CONTENT_CLASS: &str = "t-tag-content";
pub const TAG_ICON_CLASS: &str = "t-tag-iconContainer";
pub const TAG_TEXT_CLASS: &str = "t-tag-overflowingText";

pub fn tag_color_style(color: &str) -> String {
    if color == "transparent" {
        return "--tw-tag-background: transparent; --tw-tag-text: var(--t-font-color-secondary); \
                --tw-tag-border-style: solid;"
            .to_string();
    }
    format!(
        "--tw-tag-background: var(--t-tag-background-{color}); \
         --tw-tag-text: var(--t-tag-text-{color}); --tw-tag-border-style: solid;"
    )
}

pub fn is_tag_color(color: &str) -> bool {
    color == "transparent" || HUES.contains(&color)
}

#[component]
pub fn Tag(
    children: Children,
    #[prop(optional)] color: &'static str,
    #[prop(optional)] variant: Option<TagVariant>,
    #[prop(optional)] weight: Option<TagWeight>,
    #[prop(optional)] border_style: Option<TagBorderStyle>,
    #[prop(optional)] start_icon: Option<AnyView>,
    #[prop(optional, name = "truncate")] do_truncate: bool,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let color = if color.is_empty() { "gray" } else { color };
    assert!(is_tag_color(color), "unknown tag color: {color}");
    let variant = variant.unwrap_or_default();
    let weight = weight.unwrap_or_default();
    let border_style = border_style.unwrap_or(TagBorderStyle::Solid);
    let style = format!(
        "--tw-tag-background: {}; --tw-tag-text: {}; --tw-tag-border-style: {};",
        if color == "transparent" {
            "transparent".to_string()
        } else {
            format!("var(--t-tag-background-{color})")
        },
        if color == "transparent" {
            "var(--t-font-color-secondary)".to_string()
        } else {
            format!("var(--t-tag-text-{color})")
        },
        border_style.as_str()
    );
    let root_class = join_classes(&[
        Some(TAG_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let icon = start_icon;
    view! {
        <span
            class=root_class
            style=style
            data-variant=variant.as_str()
            data-weight=weight.as_str()
            data-truncate=do_truncate.then_some("")
        >
            {icon.map(|node| view! { <span class=TAG_ICON_CLASS aria-hidden="true">{node}</span> })}
            <span class=TAG_CONTENT_CLASS>
                <span class=TAG_TEXT_CLASS>{children()}</span>
            </span>
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tag_is_soft_gray_with_css_variable_colors() {
        let out = (view! { <Tag>{"Label"}</Tag> }).into_view().to_html();
        assert!(out.contains("class=\"t-tag\""), "{out}");
        assert!(out.contains("data-variant=\"soft\""), "{out}");
        assert!(
            out.contains("--tw-tag-background: var(--t-tag-background-gray)"),
            "{out}"
        );
        assert!(
            out.contains("--tw-tag-text: var(--t-tag-text-gray)"),
            "{out}"
        );
        assert!(out.contains(TAG_CONTENT_CLASS), "{out}");
        assert!(out.contains(TAG_TEXT_CLASS), "{out}");
    }

    #[test]
    fn transparent_color_uses_literal_background() {
        let out = (view! { <Tag color="transparent">{"T"}</Tag> })
            .into_view()
            .to_html();
        assert!(out.contains("--tw-tag-background: transparent"), "{out}");
        assert!(
            out.contains("--tw-tag-text: var(--t-font-color-secondary)"),
            "{out}"
        );
    }

    #[test]
    fn all_theme_colors_are_valid_tag_colors() {
        for hue in HUES {
            assert!(is_tag_color(hue), "{hue} must be a tag color");
        }
        assert!(is_tag_color("transparent"));
        assert!(!is_tag_color("not-a-color"));
    }

    #[test]
    fn variant_weight_and_border_style_are_emitted() {
        let out = (view! {
            <Tag variant=TagVariant::Outline weight=TagWeight::Medium
                 border_style=TagBorderStyle::Dashed>{"x"}</Tag>
        })
        .into_view()
        .to_html();
        assert!(out.contains("data-variant=\"outline\""), "{out}");
        assert!(out.contains("data-weight=\"medium\""), "{out}");
        assert!(out.contains("--tw-tag-border-style: dashed"), "{out}");
    }

    #[test]
    fn start_icon_renders_icon_container() {
        let out = (view! {
            <Tag start_icon=view! { <i class="ticon" /> }.into_any()>{"x"}</Tag>
        })
        .into_view()
        .to_html();
        assert!(out.contains(TAG_ICON_CLASS), "{out}");
        assert!(out.contains("ticon"), "{out}");
    }
}
