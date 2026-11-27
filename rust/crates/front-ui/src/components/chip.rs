use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ChipVariant {
    Solid,
    Soft,
    Ghost,
    #[default]
    Plain,
    Outline,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ChipShape {
    #[default]
    Square,
    Round,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ChipSize {
    #[default]
    Sm,
    Md,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ChipColor {
    #[default]
    Primary,
    Secondary,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ChipWeight {
    #[default]
    Regular,
    Medium,
}

impl ChipVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            ChipVariant::Solid => "solid",
            ChipVariant::Soft => "soft",
            ChipVariant::Ghost => "ghost",
            ChipVariant::Plain => "plain",
            ChipVariant::Outline => "outline",
        }
    }
}

impl ChipShape {
    pub fn as_str(self) -> &'static str {
        match self {
            ChipShape::Square => "square",
            ChipShape::Round => "round",
        }
    }
}

impl ChipSize {
    pub fn as_str(self) -> &'static str {
        match self {
            ChipSize::Sm => "sm",
            ChipSize::Md => "md",
        }
    }
}

impl ChipColor {
    pub fn as_str(self) -> &'static str {
        match self {
            ChipColor::Primary => "primary",
            ChipColor::Secondary => "secondary",
        }
    }
}

impl ChipWeight {
    pub fn as_str(self) -> &'static str {
        match self {
            ChipWeight::Regular => "regular",
            ChipWeight::Medium => "medium",
        }
    }
}

pub const CHIP_CLASS: &str = "t-chip";
pub const CHIP_LABEL_CLASS: &str = "t-chip-label";
pub const CHIP_END_DIVIDER_CLASS: &str = "t-chip-endElementDivider";

#[component]
pub fn Chip(
    children: Children,
    #[prop(optional)] variant: Option<ChipVariant>,
    #[prop(optional)] shape: Option<ChipShape>,
    #[prop(optional)] size: Option<ChipSize>,
    #[prop(optional)] color: Option<ChipColor>,
    #[prop(optional)] weight: Option<ChipWeight>,
    #[prop(optional)] start_element: Option<AnyView>,
    #[prop(optional)] end_element: Option<AnyView>,
    #[prop(optional)] end_element_divider: bool,
    #[prop(optional)] truncate: bool,
    #[prop(optional, name = "clickable")] is_clickable: bool,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let variant = variant.unwrap_or_default();
    let shape = shape.unwrap_or_default();
    let size = size.unwrap_or_default();
    let color = color.unwrap_or_default();
    let weight = weight.unwrap_or_default();
    let root_class = join_classes(&[
        Some(CHIP_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let start = start_element;
    let end = end_element;
    view! {
        <div
            class=root_class
            data-variant=variant.as_str()
            data-shape=shape.as_str()
            data-size=size.as_str()
            data-color=color.as_str()
            data-weight=weight.as_str()
            data-truncate=truncate.then_some("")
            data-clickable=is_clickable.then_some("")
        >
            {start}
            <span class=CHIP_LABEL_CLASS>{children()}</span>
            {if end_element_divider {
                Some(view! { <span class=CHIP_END_DIVIDER_CLASS></span> })
            } else {
                None
            }}
            {end}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(chip: impl IntoView) -> String {
        chip.into_view().to_html()
    }

    #[test]
    fn default_chip_emits_plain_square_sm_primary() {
        let out = html(view! { <Chip>{"Label"}</Chip> });
        assert!(out.contains("class=\"t-chip\""), "{out}");
        assert!(out.contains("data-variant=\"plain\""), "{out}");
        assert!(out.contains("data-shape=\"square\""), "{out}");
        assert!(out.contains("data-size=\"sm\""), "{out}");
        assert!(out.contains("data-color=\"primary\""), "{out}");
        assert!(out.contains(CHIP_LABEL_CLASS), "{out}");
        assert!(!out.contains("data-truncate"), "{out}");
    }

    #[test]
    fn every_chip_variant_and_shape_is_emitted() {
        for variant in [
            ChipVariant::Solid,
            ChipVariant::Soft,
            ChipVariant::Ghost,
            ChipVariant::Plain,
            ChipVariant::Outline,
        ] {
            let out = html(view! { <Chip variant=variant>{"v"}</Chip> });
            assert!(
                out.contains(&format!("data-variant=\"{}\"", variant.as_str())),
                "{out}"
            );
        }
        for shape in [ChipShape::Square, ChipShape::Round] {
            let out = html(view! { <Chip shape=shape>{"s"}</Chip> });
            assert!(
                out.contains(&format!("data-shape=\"{}\"", shape.as_str())),
                "{out}"
            );
        }
        let out = html(
            view! { <Chip color=ChipColor::Secondary weight=ChipWeight::Medium truncate=true>{"x"}</Chip> },
        );
        assert!(out.contains("data-color=\"secondary\""), "{out}");
        assert!(out.contains("data-weight=\"medium\""), "{out}");
        assert!(out.contains("data-truncate"), "{out}");
    }

    #[test]
    fn chip_slots_render_start_divider_and_end() {
        let out = html(view! {
            <Chip
                end_element_divider=true
                start_element=view! { <b class="chip-start">S</b> }.into_any()
                end_element=view! { <i class="chip-end">E</i> }.into_any()
            >
                {"M"}
            </Chip>
        });
        assert!(out.contains("chip-start"), "{out}");
        assert!(out.contains(CHIP_END_DIVIDER_CLASS), "{out}");
        assert!(out.contains("chip-end"), "{out}");
    }
}
