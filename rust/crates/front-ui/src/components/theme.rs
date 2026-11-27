use leptos::prelude::*;

use crate::tokens::Scheme;

pub const SQUIRCLE_SUPPORTS_SELECTOR: &str = "@supports (corner-shape: squircle)";
pub const CORNER_SHAPE_RULE: &str = "corner-shape: var(--t-corner-shape, superellipse(2.2));";

pub const SQUIRCLE_DOUBLING: [(&str, &str, &str); 6] = [
    ("--t-border-radius-xs", " 2px", " 4px"),
    ("--t-border-radius-sm", " 4px", " 8px"),
    ("--t-border-radius-md", " 8px", " 16px"),
    ("--t-border-radius-lg", " 16px", " 32px"),
    ("--t-border-radius-xl", " 20px", " 40px"),
    ("--t-border-radius-xxl", " 40px", " 80px"),
];

pub fn theme_class(scheme: Scheme) -> &'static str {
    match scheme {
        Scheme::Light => "light",
        Scheme::Dark => "dark",
    }
}

pub fn color_scheme_style(scheme: Scheme) -> String {
    format!("color-scheme: {};", scheme.as_str())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeState {
    pub scheme: Scheme,
}

impl ThemeState {
    pub fn swapped(self) -> Self {
        let scheme = match self.scheme {
            Scheme::Light => Scheme::Dark,
            Scheme::Dark => Scheme::Light,
        };
        Self { scheme }
    }

    pub fn root_class(self) -> &'static str {
        theme_class(self.scheme)
    }

    pub fn active_sheet(self) -> &'static str {
        match self.scheme {
            Scheme::Light => "tokens-light.css",
            Scheme::Dark => "tokens-dark.css",
        }
    }
}

#[component]
pub fn ThemeScope(
    scheme: Scheme,
    children: ChildrenFn,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let cls = join_classes_local(&[
        Some(theme_class(scheme)),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let style = color_scheme_style(scheme);
    view! { <div class=cls style=style>{children()}</div> }
}

fn join_classes_local(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .filter_map(|p| p.map(str::to_string))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::SQUIRCLE_RADIUS_KEYS;

    #[test]
    fn theme_scope_emits_scheme_class_that_the_sheets_key_off() {
        let app = view! { <ThemeScope scheme=Scheme::Light><span>x</span></ThemeScope> };
        let html = app.into_view().to_html();
        assert!(html.contains("class=\"light\""), "{html}");
        assert!(html.contains("color-scheme: light;"), "{html}");
        let app = view! { <ThemeScope scheme=Scheme::Dark><span>x</span></ThemeScope> };
        let html = app.into_view().to_html();
        assert!(html.contains("class=\"dark\""), "{html}");
        assert!(html.contains("color-scheme: dark;"), "{html}");
    }

    #[test]
    fn toggle_swaps_the_class_attribute() {
        let light = ThemeState {
            scheme: Scheme::Light,
        };
        let dark = light.swapped();
        assert_eq!(light.root_class(), "light");
        assert_eq!(dark.root_class(), "dark");
        assert_eq!(dark.swapped(), light);
        assert_eq!(light.active_sheet(), "tokens-light.css");
        assert_eq!(dark.active_sheet(), "tokens-dark.css");
    }

    #[test]
    fn squircle_tokens_double_radius() {
        for (key, base, doubled) in SQUIRCLE_DOUBLING {
            assert_eq!(
                crate::tokens::token(Scheme::Light, key),
                Some(base),
                "{key} base"
            );
            assert_eq!(
                crate::tokens::token(Scheme::Dark, key),
                Some(base),
                "{key} base dark"
            );
            let found = crate::tokens::squircle_table(Scheme::Light)
                .iter()
                .find(|(k, _)| *k == key)
                .unwrap_or_else(|| panic!("{key} missing from squircle table"));
            assert_eq!(found.1, doubled, "{key} doubled");
        }
    }

    #[test]
    fn squircle_keys_cover_all_radius_steps() {
        for key in SQUIRCLE_RADIUS_KEYS {
            assert!(
                SQUIRCLE_DOUBLING.iter().any(|(k, _, _)| *k == key),
                "{key} not doubled"
            );
        }
    }

    #[test]
    fn pill_and_round_never_double() {
        let sq = crate::tokens::squircle_table(Scheme::Light);
        for forbidden in ["--t-border-radius-pill", "--t-border-radius-rounded"] {
            assert!(
                !sq.iter().any(|(k, _)| *k == forbidden),
                "{forbidden} must not be doubled"
            );
        }
        for forbidden in ["sm-round", "md-round", "lg-round"] {
            let key = format!("--t-border-radius-{forbidden}");
            assert!(
                !sq.iter().any(|(k, _)| *k == key),
                "{key} must not be doubled"
            );
        }
    }
}
