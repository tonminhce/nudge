use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SeparatorOrientation {
    #[default]
    Horizontal,
    Vertical,
}

impl SeparatorOrientation {
    pub fn as_str(self) -> &'static str {
        match self {
            SeparatorOrientation::Horizontal => "horizontal",
            SeparatorOrientation::Vertical => "vertical",
        }
    }
}

pub const SEPARATOR_CLASS: &str = "t-separator";

#[component]
pub fn Separator(
    #[prop(optional)] orientation: Option<SeparatorOrientation>,
    #[prop(default = true)] decorative: bool,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let orientation = orientation.unwrap_or_default();
    let root_class = join_classes(&[
        Some(SEPARATOR_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    view! {
        <div
            class=root_class
            data-orientation=orientation.as_str()
            role=(!decorative).then_some("separator")
            aria-orientation=(!decorative).then_some(orientation.as_str())
            aria-hidden=decorative.then_some("true")
        ></div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_separator_is_horizontal_and_decorative() {
        let out = (view! { <Separator/> }).into_view().to_html();
        assert!(out.contains("class=\"t-separator\""), "{out}");
        assert!(out.contains("data-orientation=\"horizontal\""), "{out}");
        assert!(out.contains("aria-hidden=\"true\""), "{out}");
        assert!(!out.contains("role="), "{out}");
        assert!(!out.contains("aria-orientation="), "{out}");
    }

    #[test]
    fn semantic_vertical_separator_exposes_orientation() {
        let out = (view! {
            <Separator orientation=SeparatorOrientation::Vertical decorative=false class="divider"/>
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-separator divider\""), "{out}");
        assert!(out.contains("data-orientation=\"vertical\""), "{out}");
        assert!(out.contains("role=\"separator\""), "{out}");
        assert!(out.contains("aria-orientation=\"vertical\""), "{out}");
        assert!(!out.contains("aria-hidden="), "{out}");
    }

    #[test]
    fn separator_props_builder_accepts_orientation_and_decorative_flag() {
        let props = SeparatorProps::builder()
            .orientation(SeparatorOrientation::Vertical)
            .decorative(false)
            .build();
        let out = Separator(props).into_view().to_html();
        assert!(out.contains("role=\"separator\""), "{out}");
        assert!(out.contains("aria-orientation=\"vertical\""), "{out}");
    }
}
