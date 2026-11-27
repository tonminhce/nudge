pub mod generated;

#[cfg(test)]
mod tests;

use generated::{
    DARK_SQUIRCLE, DARK_TOKENS, LAYOUT_TOKENS, LIGHT_SQUIRCLE, LIGHT_TOKENS, TOKEN_COUNT_COLOR,
    TOKEN_COUNT_DARK, TOKEN_COUNT_HUES, TOKEN_COUNT_LIGHT, TOKEN_STEPS_PER_HUE,
};

pub const HUES: [&str; TOKEN_COUNT_HUES] = [
    "red",
    "ruby",
    "crimson",
    "tomato",
    "orange",
    "amber",
    "yellow",
    "lime",
    "grass",
    "green",
    "jade",
    "mint",
    "turquoise",
    "cyan",
    "sky",
    "blue",
    "iris",
    "violet",
    "purple",
    "plum",
    "pink",
    "bronze",
    "gold",
    "brown",
    "gray",
];

pub const HUE_STEPS: [u8; TOKEN_STEPS_PER_HUE] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

pub const SPACING_STEPS: [u8; 33] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32,
];

pub const SQUIRCLE_RADIUS_KEYS: [&str; 6] = [
    "--t-border-radius-xs",
    "--t-border-radius-sm",
    "--t-border-radius-md",
    "--t-border-radius-lg",
    "--t-border-radius-xl",
    "--t-border-radius-xxl",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scheme {
    Light,
    Dark,
}

impl Scheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Scheme::Light => "light",
            Scheme::Dark => "dark",
        }
    }
}

pub fn token_table(scheme: Scheme) -> &'static [(&'static str, &'static str)] {
    match scheme {
        Scheme::Light => LIGHT_TOKENS,
        Scheme::Dark => DARK_TOKENS,
    }
}

pub fn squircle_table(scheme: Scheme) -> &'static [(&'static str, &'static str)] {
    match scheme {
        Scheme::Light => LIGHT_SQUIRCLE,
        Scheme::Dark => DARK_SQUIRCLE,
    }
}

pub fn layout_table() -> &'static [(&'static str, &'static str)] {
    LAYOUT_TOKENS
}

pub fn layout(name: &str) -> Option<&'static str> {
    LAYOUT_TOKENS
        .iter()
        .find(|(k, _)| *k == name)
        .map(|&(_, v)| v)
}

pub fn token(scheme: Scheme, name: &str) -> Option<&'static str> {
    token_table(scheme)
        .iter()
        .find(|(k, _)| *k == name)
        .map(|&(_, v)| v)
}

pub fn color_count() -> usize {
    TOKEN_COUNT_COLOR
}

pub fn light_count() -> usize {
    TOKEN_COUNT_LIGHT
}

pub fn dark_count() -> usize {
    TOKEN_COUNT_DARK
}

pub fn color_name(hue: &str, step: u8) -> Option<String> {
    if !HUES.contains(&hue) {
        return None;
    }
    if step > 12 {
        return None;
    }
    if step == 0 {
        return Some(format!("--t-color-{hue}"));
    }
    Some(format!("--t-color-{hue}{step}"))
}

pub fn color(scheme: Scheme, hue: &str, step: u8) -> Option<&'static str> {
    color_name(hue, step).and_then(|name| token(scheme, &name))
}

pub fn spacing_name(step: u8) -> String {
    match step {
        0 => "--t-spacing-0".to_string(),
        _ => format!("--t-spacing-{step}"),
    }
}

pub fn spacing(scheme: Scheme, step: u8) -> Option<&'static str> {
    token(scheme, &spacing_name(step))
}

pub fn radius(scheme: Scheme, size: &str) -> Option<&'static str> {
    token(scheme, &format!("--t-border-radius-{size}"))
}

pub fn shadow(scheme: Scheme, name: &str) -> Option<&'static str> {
    token(scheme, &format!("--t-box-shadow-{name}"))
}

pub fn font(scheme: Scheme, name: &str) -> Option<&'static str> {
    token(scheme, &format!("--t-font-{name}"))
}

pub fn font_size(scheme: Scheme, step: &str) -> Option<&'static str> {
    token(scheme, &format!("--t-font-size-{step}"))
}

pub fn font_weight(scheme: Scheme, weight: &str) -> Option<&'static str> {
    token(scheme, &format!("--t-font-weight-{weight}"))
}

fn render_block(selector: &str, decls: &[(&str, &str)]) -> String {
    let mut out = format!("  {selector} {{\n");
    for (key, value) in decls {
        out.push_str("    ");
        out.push_str(key);
        out.push(':');
        out.push_str(value);
        out.push_str(";\n");
    }
    out.push_str("  }\n");
    out
}

fn render_squircle(scheme: Scheme, decls: &[(&str, &str)]) -> String {
    let mut out = String::from("  @supports (corner-shape: squircle) {\n");
    out.push_str(&render_block(&format!(".{}", scheme.as_str()), decls));
    out.push('\n');
    out.push_str("    *,\n    *::before,\n    *::after {\n      corner-shape: var(--t-corner-shape, superellipse(2.2));\n    }\n");
    out.push_str("  }\n");
    out
}

pub fn generate_css(scheme: Scheme) -> String {
    let mut out = String::from("@layer twenty-ui {\n");
    out.push_str(&render_block(
        &format!(".{}", scheme.as_str()),
        token_table(scheme),
    ));
    out.push('\n');
    out.push_str(&render_squircle(scheme, squircle_table(scheme)));
    out.push_str(&render_block(".light,\n  .dark", LAYOUT_TOKENS));
    out.push_str("}\n");
    out
}
