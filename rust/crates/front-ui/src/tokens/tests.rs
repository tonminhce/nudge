use crate::tokens::generated::{DARK_TOKENS, LIGHT_TOKENS};
use crate::tokens::{
    color_count, dark_count, generate_css, light_count, squircle_table, token, Scheme, HUES,
    SQUIRCLE_RADIUS_KEYS,
};

fn fixture(name: &str) -> String {
    let path = format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read fixture {path}: {e}"))
}

fn scheme_region(css: &str, scheme: Scheme) -> &str {
    let open = format!("  .{} {{\n", scheme.as_str());
    let start = css
        .find(&open)
        .unwrap_or_else(|| panic!("no .{} block", scheme.as_str()))
        + open.len();
    let rest = &css[start..];
    let end = rest
        .find("\n  }\n")
        .unwrap_or_else(|| panic!("no end of .{} block", scheme.as_str()));
    &rest[..end]
}

fn squircle_region(css: &str, scheme: Scheme) -> &str {
    let open = format!(
        "@supports (corner-shape: squircle) {{\n    .{} {{\n",
        scheme.as_str()
    );
    let start = css
        .find(&open)
        .unwrap_or_else(|| panic!("no squircle block for {}", scheme.as_str()))
        + open.len();
    let rest = &css[start..];
    let end = rest
        .find("\n    }\n")
        .unwrap_or_else(|| panic!("no end of squircle block for {}", scheme.as_str()));
    &rest[..end]
}

fn reconstruct(decls: &[(&str, &str)]) -> String {
    decls
        .iter()
        .map(|(k, v)| format!("    {k}:{v};"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn reconstruct_squircle(decls: &[(&str, &str)]) -> String {
    decls
        .iter()
        .map(|(k, v)| format!("      {k}:{v};"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn light_declaration_block_is_byte_equal_to_fixture() {
    let css = fixture("theme-light.css");
    let parsed = scheme_region(&css, Scheme::Light);
    let table: &[(&str, &str)] = LIGHT_TOKENS;
    assert_eq!(table.len(), 1003);
    assert_eq!(parsed, reconstruct(table));
}

#[test]
fn dark_declaration_block_is_byte_equal_to_fixture() {
    let css = fixture("theme-dark.css");
    let parsed = scheme_region(&css, Scheme::Dark);
    let table: &[(&str, &str)] = DARK_TOKENS;
    assert_eq!(table.len(), 1003);
    assert_eq!(parsed, reconstruct(table));
}

#[test]
fn squircle_blocks_are_byte_equal_to_fixture() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let css = fixture(&format!("theme-{}.css", scheme.as_str()));
        let parsed = squircle_region(&css, scheme);
        let table = squircle_table(scheme);
        assert_eq!(
            parsed,
            reconstruct_squircle(table),
            "{scheme:?} squircle block"
        );
        assert_eq!(table.len(), SQUIRCLE_RADIUS_KEYS.len());
    }
}

#[test]
fn generated_css_round_trips_through_the_emitter() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let css = fixture(&format!("theme-{}.css", scheme.as_str()));
        let open = format!("  .{} {{\n", scheme.as_str());
        let start = css.find(&open).unwrap() + open.len();
        let end = css[start..].find("\n  }\n").unwrap();
        let generated = generate_css(scheme);
        let region = &css[start..start + end];
        assert!(
            generated.contains(region),
            "{scheme:?}: emitted CSS does not contain the fixture declaration block verbatim"
        );
    }
}

#[test]
fn token_counts_match_upstream() {
    assert_eq!(light_count(), 1003);
    assert_eq!(dark_count(), 1003);
    assert_eq!(color_count(), 745);
}

#[test]
fn semantic_anchor_values_match_research_doc() {
    let light = Scheme::Light;
    let dark = Scheme::Dark;
    assert_eq!(
        token(light, "--t-background-primary"),
        Some(" color(display-p3 1 1 1)")
    );
    assert_eq!(
        token(light, "--t-background-secondary"),
        Some(" color(display-p3 0.988 0.988 0.988)")
    );
    assert_eq!(
        token(light, "--t-background-tertiary"),
        Some(" color(display-p3 0.945 0.945 0.945)")
    );
    assert_eq!(
        token(light, "--t-background-quaternary"),
        Some(" color(display-p3 0.922 0.922 0.922)")
    );
    assert_eq!(
        token(dark, "--t-background-primary"),
        Some(" color(display-p3 0.09 0.09 0.09)")
    );
    assert_eq!(
        token(dark, "--t-background-tertiary"),
        Some(" color(display-p3 0.114 0.114 0.114)")
    );
    assert_eq!(
        token(light, "--t-font-color-primary"),
        Some(" color(display-p3 0.2 0.2 0.2)")
    );
    assert_eq!(
        token(light, "--t-font-color-secondary"),
        Some(" color(display-p3 0.4 0.4 0.4)")
    );
    assert_eq!(
        token(light, "--t-font-color-tertiary"),
        Some(" color(display-p3 0.6 0.6 0.6)")
    );
    assert_eq!(
        token(dark, "--t-font-color-primary"),
        Some(" color(display-p3 0.922 0.922 0.922)")
    );
    assert_eq!(
        token(dark, "--t-font-color-secondary"),
        Some(" color(display-p3 0.702 0.702 0.702)")
    );
    assert_eq!(
        token(dark, "--t-font-color-tertiary"),
        Some(" color(display-p3 0.506 0.506 0.506)")
    );
    assert_eq!(
        token(light, "--t-border-color-strong"),
        Some(" color(display-p3 0.839 0.839 0.839)")
    );
    assert_eq!(
        token(dark, "--t-border-color-strong"),
        Some(" color(display-p3 0.282 0.282 0.282)")
    );
    assert_eq!(
        token(light, "--t-border-color-medium"),
        Some(" color(display-p3 0.922 0.922 0.922)")
    );
    assert_eq!(
        token(dark, "--t-border-color-medium"),
        Some(" color(display-p3 0.133 0.133 0.133)")
    );
    assert_eq!(
        token(light, "--t-accent-primary"),
        Some(" color(display-p3 0.831 0.87 1)")
    );
    assert_eq!(
        token(dark, "--t-accent-primary"),
        Some(" color(display-p3 0.163 0.22 0.439)")
    );
    assert_eq!(
        token(light, "--t-accent-accent1"),
        Some(" color(display-p3 0.992 0.992 0.996)")
    );
    assert_eq!(
        token(light, "--t-accent-accent9"),
        Some(" color(display-p3 0.276 0.384 0.837)")
    );
    assert_eq!(
        token(dark, "--t-accent-accent9"),
        Some(" color(display-p3 0.276 0.384 0.837)")
    );
    assert_eq!(
        token(light, "--t-accent-accent12"),
        Some(" color(display-p3 0.133 0.175 0.348)")
    );
    assert_eq!(
        token(dark, "--t-accent-accent1"),
        Some(" color(display-p3 0.068 0.074 0.118)")
    );
    assert_eq!(
        token(dark, "--t-accent-accent12"),
        Some(" color(display-p3 0.848 0.881 0.99)")
    );
}

#[test]
fn scale_anchors_match_research_doc() {
    assert_eq!(
        token(Scheme::Light, "--t-spacing-multiplicator"),
        Some(" 4")
    );
    assert_eq!(token(Scheme::Light, "--t-spacing-0"), Some(" 0px"));
    assert_eq!(token(Scheme::Light, "--t-spacing-1"), Some(" 4px"));
    assert_eq!(token(Scheme::Light, "--t-spacing-2"), Some(" 8px"));
    assert_eq!(token(Scheme::Light, "--t-spacing-4"), Some(" 16px"));
    assert_eq!(token(Scheme::Light, "--t-spacing-8"), Some(" 32px"));
    assert_eq!(token(Scheme::Light, "--t-spacing-16"), Some(" 64px"));
    assert_eq!(token(Scheme::Light, "--t-spacing-32"), Some(" 128px"));
    assert_eq!(token(Scheme::Light, "--t-spacing-0_5"), Some(" 2px"));
    assert_eq!(token(Scheme::Light, "--t-spacing-1_5"), Some(" 6px"));
    assert_eq!(token(Scheme::Light, "--t-border-radius-xs"), Some(" 2px"));
    assert_eq!(token(Scheme::Light, "--t-border-radius-sm"), Some(" 4px"));
    assert_eq!(token(Scheme::Light, "--t-border-radius-md"), Some(" 8px"));
    assert_eq!(token(Scheme::Light, "--t-border-radius-lg"), Some(" 16px"));
    assert_eq!(token(Scheme::Light, "--t-border-radius-xl"), Some(" 20px"));
    assert_eq!(token(Scheme::Light, "--t-border-radius-xxl"), Some(" 40px"));
    assert_eq!(
        token(Scheme::Light, "--t-border-radius-pill"),
        Some(" 999px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-border-radius-rounded"),
        Some(" 100%")
    );
    assert_eq!(
        token(Scheme::Light, "--t-border-radius-sm-round"),
        Some(" 4px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-border-radius-md-round"),
        Some(" 8px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-border-radius-lg-round"),
        Some(" 12px")
    );
    assert_eq!(token(Scheme::Light, "--t-font-size-xxs"), Some(" 0.625rem"));
    assert_eq!(token(Scheme::Light, "--t-font-size-xs"), Some(" 0.85rem"));
    assert_eq!(token(Scheme::Light, "--t-font-size-sm"), Some(" 0.92rem"));
    assert_eq!(token(Scheme::Light, "--t-font-size-md"), Some(" 1rem"));
    assert_eq!(token(Scheme::Light, "--t-font-size-lg"), Some(" 1.23rem"));
    assert_eq!(token(Scheme::Light, "--t-font-size-xl"), Some(" 1.54rem"));
    assert_eq!(token(Scheme::Light, "--t-font-size-xxl"), Some(" 1.85rem"));
    assert_eq!(
        token(Scheme::Light, "--t-font-weight-regular"),
        Some(" 400")
    );
    assert_eq!(token(Scheme::Light, "--t-font-weight-medium"), Some(" 500"));
    assert_eq!(
        token(Scheme::Light, "--t-font-weight-semi-bold"),
        Some(" 600")
    );
    assert_eq!(
        token(Scheme::Light, "--t-font-family"),
        Some(" Inter, sans-serif")
    );
    assert_eq!(
        token(Scheme::Light, "--t-code-font-family"),
        Some(" DM Mono")
    );
    assert_eq!(
        token(Scheme::Light, "--t-table-horizontal-cell-margin"),
        Some(" 8px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-table-checkbox-column-width"),
        Some(" 32px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-table-horizontal-cell-padding"),
        Some(" 8px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-modal-size-sm-width"),
        Some(" 300px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-modal-size-compact-width"),
        Some(" 360px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-modal-size-md-width"),
        Some(" 400px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-modal-size-lg-width"),
        Some(" 53%")
    );
    assert_eq!(
        token(Scheme::Light, "--t-modal-size-xl-width"),
        Some(" 1200px")
    );
    assert_eq!(
        token(Scheme::Light, "--t-modal-size-xl-height"),
        Some(" 800px")
    );
}

#[test]
fn squircle_radii_double() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let doubled = [
            ("--t-border-radius-xs", " 4px"),
            ("--t-border-radius-sm", " 8px"),
            ("--t-border-radius-md", " 16px"),
            ("--t-border-radius-lg", " 32px"),
            ("--t-border-radius-xl", " 40px"),
            ("--t-border-radius-xxl", " 80px"),
        ];
        for (key, value) in doubled {
            let found = squircle_table(scheme)
                .iter()
                .find(|(k, _)| *k == key)
                .unwrap_or_else(|| panic!("{scheme:?} missing squircle {key}"));
            assert_eq!(found.1, value, "{scheme:?}: {key} does not double");
        }
    }
    assert_eq!(
        token(Scheme::Light, "--t-border-radius-pill"),
        Some(" 999px")
    );
    assert!(squircle_table(Scheme::Light)
        .iter()
        .all(|(k, _)| SQUIRCLE_RADIUS_KEYS.contains(k)));
}

#[test]
fn hue_ramp_is_complete() {
    for hue in HUES {
        for step in 0u8..=12 {
            let name = if step == 0 {
                format!("--t-color-{hue}")
            } else {
                format!("--t-color-{hue}{step}")
            };
            assert!(
                token(Scheme::Light, &name).is_some(),
                "missing light token {name}"
            );
            assert!(
                token(Scheme::Dark, &name).is_some(),
                "missing dark token {name}"
            );
        }
    }
}

fn p3_first_channel(value: &str) -> f32 {
    let inner = value
        .trim()
        .strip_prefix("color(display-p3 ")
        .unwrap_or_else(|| panic!("not a display-p3 color: {value}"))
        .split('/')
        .next()
        .unwrap();
    inner
        .split_whitespace()
        .next()
        .unwrap()
        .parse::<f32>()
        .unwrap_or_else(|e| panic!("bad p3 channel in {value}: {e}"))
}

#[test]
fn ramps_invert_between_schemes() {
    assert_ne!(
        token(Scheme::Light, "--t-color-blue1"),
        token(Scheme::Dark, "--t-color-blue1")
    );
    assert_ne!(
        token(Scheme::Light, "--t-color-gray12"),
        token(Scheme::Dark, "--t-color-gray12")
    );
    assert_eq!(
        token(Scheme::Light, "--t-accent-accent9"),
        token(Scheme::Dark, "--t-accent-accent9"),
        "accent9 product blue is identical in both schemes"
    );
}

#[test]
fn gray_ramp_runs_light_to_dark_in_light_and_the_reverse_in_dark() {
    assert!(p3_first_channel(token(Scheme::Light, "--t-color-gray1").unwrap()) > 0.9);
    assert!(p3_first_channel(token(Scheme::Light, "--t-color-gray12").unwrap()) < 0.3);
    assert!(p3_first_channel(token(Scheme::Dark, "--t-color-gray1").unwrap()) < 0.15);
    assert!(p3_first_channel(token(Scheme::Dark, "--t-color-gray12").unwrap()) > 0.9);
    assert!(
        p3_first_channel(token(Scheme::Light, "--t-color-gray1").unwrap())
            > p3_first_channel(token(Scheme::Light, "--t-color-gray12").unwrap()),
        "light ramp starts light and ends dark"
    );
    assert!(
        p3_first_channel(token(Scheme::Dark, "--t-color-gray1").unwrap())
            < p3_first_channel(token(Scheme::Dark, "--t-color-gray12").unwrap()),
        "dark ramp starts dark and ends light"
    );
}

#[test]
fn generated_light_and_dark_tables_share_the_same_token_order() {
    let light_keys: Vec<&str> = LIGHT_TOKENS.iter().map(|(k, _)| *k).collect();
    let dark_keys: Vec<&str> = DARK_TOKENS.iter().map(|(k, _)| *k).collect();
    assert_eq!(light_keys, dark_keys);
}

#[test]
fn committed_generated_sheets_match_the_emitter() {
    for scheme in [Scheme::Light, Scheme::Dark] {
        let path = format!(
            "{}/theme/generated/tokens-{}.css",
            env!("CARGO_MANIFEST_DIR"),
            scheme.as_str()
        );
        let committed = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read committed sheet {path}: {e}"));
        assert_eq!(
            committed,
            generate_css(scheme),
            "{path} is stale; regenerate with `cargo run --bin gen-tokens -p front-ui`"
        );
        assert!(
            !committed.contains("/*"),
            "{path} must carry no comments (R-001)"
        );
    }
}
