use leptos::prelude::*;

use crate::components::button::join_classes;
use crate::tokens::{color, Scheme, HUES};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AvatarSize {
    Xxl,
    Xl,
    Lg,
    #[default]
    Md,
    Sm,
    Xs,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AvatarShape {
    #[default]
    Square,
    RoundedSquare,
    Circle,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AvatarVariant {
    #[default]
    Soft,
    Outline,
}

impl AvatarSize {
    pub fn as_str(self) -> &'static str {
        match self {
            AvatarSize::Xxl => "xxl",
            AvatarSize::Xl => "xl",
            AvatarSize::Lg => "lg",
            AvatarSize::Md => "md",
            AvatarSize::Sm => "sm",
            AvatarSize::Xs => "xs",
        }
    }

    pub fn width(self) -> &'static str {
        match self {
            AvatarSize::Xxl => "40px",
            AvatarSize::Xl => "32px",
            AvatarSize::Lg => "24px",
            AvatarSize::Md => "16px",
            AvatarSize::Sm => "14px",
            AvatarSize::Xs => "12px",
        }
    }

    pub fn font_size(self) -> &'static str {
        match self {
            AvatarSize::Xxl => "16px",
            AvatarSize::Xl => "16px",
            AvatarSize::Lg => "13px",
            AvatarSize::Md => "12px",
            AvatarSize::Sm => "10px",
            AvatarSize::Xs => "8px",
        }
    }
}

impl AvatarShape {
    pub fn as_str(self) -> &'static str {
        match self {
            AvatarShape::Square => "square",
            AvatarShape::RoundedSquare => "rounded-square",
            AvatarShape::Circle => "circle",
        }
    }
}

impl AvatarVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            AvatarVariant::Soft => "soft",
            AvatarVariant::Outline => "outline",
        }
    }
}

pub const AVATAR_ROOT_CLASS: &str = "t-avatar-root";
pub const AVATAR_IMAGE_CLASS: &str = "t-avatar-image";
pub const AVATAR_FALLBACK_CLASS: &str = "t-avatar-fallback";

pub fn get_avatar_initial(name: &str) -> String {
    name.trim()
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_default()
}

fn to_int32(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    let n = x.trunc();
    let m = n.rem_euclid(4294967296.0);
    if m >= 2147483648.0 {
        (m - 4294967296.0) as i32
    } else {
        m as i32
    }
}

fn js_hash(s: &str) -> f64 {
    let mut h: f64 = 0.0;
    for unit in s.encode_utf16() {
        let c = unit as f64;
        let shifted = ((to_int32(h) as u32).wrapping_mul(32)) as i32;
        h = c + (shifted as f64 - h);
    }
    h
}

pub fn string_to_theme_color(string: &str) -> &'static str {
    let index = js_hash(string).abs().rem_euclid(HUES.len() as f64) as usize;
    HUES[index]
}

pub fn string_to_theme_color_p3(string: &str, variant: u8, scheme: Scheme) -> Option<&'static str> {
    let hue = string_to_theme_color(string);
    color(scheme, hue, variant)
}

#[component]
pub fn Avatar(
    #[prop(optional)] name: &'static str,
    #[prop(optional)] size: Option<AvatarSize>,
    #[prop(optional)] shape: Option<AvatarShape>,
    #[prop(optional)] variant: Option<AvatarVariant>,
    #[prop(optional)] image_url: &'static str,
    #[prop(optional)] pulsing: bool,
    #[prop(optional)] ring: bool,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let size = size.unwrap_or_default();
    let shape = shape.unwrap_or_default();
    let variant = variant.unwrap_or_default();
    let initial = get_avatar_initial(name);
    let hue = string_to_theme_color(name);
    let text_variant: u8 = 12;
    let background_variant: u8 = if variant == AvatarVariant::Outline {
        5
    } else {
        4
    };
    let border_variant: u8 = 6;
    let root_class = join_classes(&[
        Some(AVATAR_ROOT_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let style = format!(
        "block-size: {w}; inline-size: {w}; font-size: {f}; \
         --tw-avatar-color: var(--t-color-{hue}{tv}); \
         --tw-avatar-background: var(--t-color-{hue}{bv}); \
         --tw-avatar-border: {border};",
        w = size.width(),
        f = size.font_size(),
        tv = text_variant,
        bv = background_variant,
        border = if variant == AvatarVariant::Outline {
            format!("1px solid var(--t-color-{hue}{bvv})", bvv = border_variant)
        } else {
            "none".to_string()
        },
    );
    let img = !image_url.is_empty();
    view! {
        <div
            class=root_class
            style=style
            data-size=size.as_str()
            data-shape=shape.as_str()
            data-pulsing=pulsing.then_some("")
            data-ring=ring.then_some("")
        >
            {img.then(|| view! { <img class=AVATAR_IMAGE_CLASS alt=name src=image_url /> })}
            <span class=AVATAR_FALLBACK_CLASS aria-hidden="true">{initial}</span>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_initial_matches_upstream_get_avatar_initial() {
        assert_eq!(get_avatar_initial("tony"), "T".to_string());
        assert_eq!(get_avatar_initial("  alice "), "A".to_string());
        assert_eq!(get_avatar_initial(""), "".to_string());
    }

    #[test]
    fn seeded_color_matches_upstream_hash() {
        assert_eq!(string_to_theme_color("tony"), "grass");
        assert_eq!(string_to_theme_color("Alice"), "purple");
        assert_eq!(string_to_theme_color("Bob"), "blue");
        assert_eq!(string_to_theme_color(""), "red");
        assert_eq!(string_to_theme_color("Company X"), "brown");
        assert_eq!(string_to_theme_color("jane.doe@example.com"), "blue");
    }

    #[test]
    fn seeded_colors_resolve_against_the_token_sheets() {
        assert_eq!(
            string_to_theme_color_p3("tony", 12, Scheme::Light),
            color(Scheme::Light, "grass", 12)
        );
        assert_eq!(
            string_to_theme_color_p3("Alice", 4, Scheme::Dark),
            color(Scheme::Dark, "purple", 4)
        );
        for hue in HUES {
            assert!(color(Scheme::Light, hue, 4).is_some(), "missing {hue}4");
            assert!(color(Scheme::Dark, hue, 4).is_some(), "missing dark {hue}4");
        }
    }

    #[test]
    fn avatar_size_metrics_match_upstream() {
        let expected = [
            (AvatarSize::Xxl, "40px", "16px"),
            (AvatarSize::Xl, "32px", "16px"),
            (AvatarSize::Lg, "24px", "13px"),
            (AvatarSize::Md, "16px", "12px"),
            (AvatarSize::Sm, "14px", "10px"),
            (AvatarSize::Xs, "12px", "8px"),
        ];
        for (size, width, font) in expected {
            assert_eq!(size.width(), width);
            assert_eq!(size.font_size(), font);
        }
    }

    #[test]
    fn avatar_emits_classes_sizes_and_seed_colors() {
        let out = (view! { <Avatar name="tony" size=AvatarSize::Lg shape=AvatarShape::Circle ring=true pulsing=true/> })
            .into_view()
            .to_html();
        assert!(out.contains(AVATAR_ROOT_CLASS), "{out}");
        assert!(out.contains(AVATAR_FALLBACK_CLASS), "{out}");
        assert!(out.contains("data-size=\"lg\""), "{out}");
        assert!(out.contains("data-shape=\"circle\""), "{out}");
        assert!(out.contains("data-pulsing"), "{out}");
        assert!(out.contains("data-ring"), "{out}");
        assert!(out.contains("block-size: 24px"), "{out}");
        assert!(
            out.contains("--tw-avatar-color: var(--t-color-grass12)"),
            "{out}"
        );
        assert!(
            out.contains("--tw-avatar-background: var(--t-color-grass4)"),
            "{out}"
        );
    }

    #[test]
    fn outline_variant_adds_a_border() {
        let out = (view! { <Avatar name="Bob" variant=AvatarVariant::Outline/> })
            .into_view()
            .to_html();
        assert!(
            out.contains("--tw-avatar-background: var(--t-color-blue5)"),
            "{out}"
        );
        assert!(
            out.contains("--tw-avatar-border: 1px solid var(--t-color-blue6)"),
            "{out}"
        );
    }

    #[test]
    fn image_slot_renders_with_fallback_behind() {
        let out = (view! { <Avatar name="A" image_url="/a.png"/> })
            .into_view()
            .to_html();
        assert!(out.contains(AVATAR_IMAGE_CLASS), "{out}");
        assert!(out.contains("src=\"/a.png\""), "{out}");
        assert!(out.contains(AVATAR_FALLBACK_CLASS), "{out}");
    }
}
