use leptos::prelude::view;
use leptos::prelude::RenderHtml;
use leptos::IntoView;

use crate::icons::custom_icon_def;
use crate::icons::icon_class;
use crate::icons::icon_def;
use crate::icons::object_icon;
use crate::icons::render_custom_icon;
use crate::icons::render_icon;
use crate::icons::Icon;
use crate::icons::IconAddressBook;
use crate::icons::IconBrandAnthropic;
use crate::icons::IconChartBarHorizontal;
use crate::icons::IconLock;
use crate::icons::IconLockCustom;
use crate::icons::IconSparkle2;
use crate::icons::IconStroke;
use crate::icons::IconTwentyStar;
use crate::icons::IllustrationIconUser;
use crate::icons::ThinkingOrbitLoaderIcon;
use crate::icons::CUSTOM_ICON_CATALOG_TAG;
use crate::icons::CUSTOM_ICON_COUNT;
use crate::icons::CUSTOM_ICON_DEFS;
use crate::icons::ICON_CATALOG_TAG;
use crate::icons::ICON_COUNT;
use crate::icons::ICON_DEFS;
use crate::icons::OBJECT_ICONS;
use crate::UPSTREAM_TAG;

fn d_values(svg: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remaining = svg;
    while let Some(index) = remaining.find("d=\"") {
        let rest = &remaining[index + 3..];
        let Some(end) = rest.find('"') else {
            break;
        };
        let value = rest[..end].to_string();
        if value != "M0 0h24v24H0z" {
            values.push(value);
        }
        remaining = &rest[end + 1..];
    }
    values
}

#[test]
fn icon_count_matches_the_complete_upstream_catalog() {
    assert_eq!(ICON_COUNT, 4187);
    assert_eq!(ICON_CATALOG_TAG, "twenty-v2.61.0-tabler-3.41.1");
}

#[test]
fn catalog_is_sorted_for_binary_search_without_duplicate_names() {
    for pair in ICON_DEFS.windows(2) {
        assert!(pair[0].tabler_name < pair[1].tabler_name);
    }
    for def in ICON_DEFS {
        assert!(
            std::ptr::eq(icon_def(def.tabler_name).unwrap(), def),
            "{}",
            def.tabler_name
        );
    }
}

#[test]
fn object_icons_match_the_twenty_icon_dictionary() {
    let expected = [
        ("person", "user", "IconUser"),
        ("company", "building-skyscraper", "IconBuildingSkyscraper"),
        ("opportunity", "target-arrow", "IconTargetArrow"),
        ("task", "checkbox", "IconCheckbox"),
        ("note", "notes", "IconNotes"),
        ("workflow", "settings-automation", "IconSettingsAutomation"),
        ("agent", "lego", "IconLego"),
    ];
    for (key, tabler, upstream) in expected {
        assert_eq!(object_icon(key), Some(tabler), "{key} maps to {upstream}");
        assert!(
            icon_def(tabler).is_some(),
            "{key}: tabler icon {tabler} must exist in ICON_DEFS"
        );
    }
    assert!(object_icon("not-an-object").is_none());
}

#[test]
fn every_object_icon_entry_resolves_to_a_real_icon_def() {
    for (key, tabler) in OBJECT_ICONS {
        assert!(
            icon_def(tabler).is_some(),
            "{key}: mapped icon {tabler} missing"
        );
    }
}

#[test]
fn every_fixture_resolves_to_an_upstream_icon() {
    let directory = format!("{}/fixtures/icons", env!("CARGO_MANIFEST_DIR"));
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("svg") {
            continue;
        }
        let name = path.file_stem().unwrap().to_str().unwrap();
        let def = icon_def(name).unwrap_or_else(|| panic!("missing port of {name}"));
        let svg = std::fs::read_to_string(&path).unwrap();
        assert!(!def.inner_svg.is_empty(), "{name}");
        assert!(
            svg.contains(&format!("viewBox=\"{}\"", def.view_box)),
            "{name}: view box not in fixture"
        );
    }
}

#[test]
fn standard_icons_do_not_render_the_invisible_fixture_background_as_a_border() {
    let def = icon_def("user").expect("user icon present");
    assert_eq!(def.class_name, "tabler-icon tabler-icon-user ");
    assert!(!def.inner_svg.contains("M0 0h24v24H0z"));
    assert_eq!(d_values(def.inner_svg).len(), 2);
    let html = render_icon(def, 16, IconStroke::Sm, "currentColor");
    assert!(!html.contains("M0 0h24v24H0z"), "{html}");
}

#[test]
fn rendered_svg_uses_the_upstream_root_attributes_and_inner_markup() {
    let def = icon_def("user").expect("user icon present");
    let html = render_icon(def, 16, IconStroke::Sm, "currentColor");
    assert!(
        html.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""),
        "{html}"
    );
    assert!(html.contains("viewBox=\"0 0 24 24\""), "{html}");
    assert!(html.contains("fill=\"none\""), "{html}");
    assert!(html.contains("stroke=\"currentColor\""), "{html}");
    assert!(html.contains("stroke-width=\"1.6\""), "{html}");
    assert!(html.contains("stroke-linecap=\"round\""), "{html}");
    assert!(
        html.contains(&format!("class=\"{}\"", icon_class(def))),
        "{html}"
    );
    assert!(html.contains(def.inner_svg), "{html}");
}

#[test]
fn animated_loader_keeps_the_upstream_smil_animation() {
    let html = view! { <ThinkingOrbitLoaderIcon size=14 /> }
        .into_view()
        .to_html();
    assert!(html.contains("stroke-dashoffset"), "{html}");
    assert!(html.contains("values=\"0;-100\""), "{html}");
    assert!(html.contains("dur=\"1.05s\""), "{html}");
    assert_eq!(html.matches("repeatCount=\"indefinite\"").count(), 2);
}

#[test]
fn custom_catalog_contains_brand_and_illustration_icons() {
    assert_eq!(CUSTOM_ICON_COUNT, 46);
    for pair in CUSTOM_ICON_DEFS.windows(2) {
        assert!(pair[0].name < pair[1].name);
    }
    for def in CUSTOM_ICON_DEFS {
        assert!(std::ptr::eq(custom_icon_def(def.name).unwrap(), def));
    }
    for name in [
        "address-book",
        "gmail",
        "google",
        "microsoft-outlook",
        "anthropic",
        "openai",
        "illustration-user",
        "many-to-one",
        "twenty-star-filled",
    ] {
        assert!(custom_icon_def(name).is_some(), "{name}");
    }
}

#[test]
fn custom_icon_render_preserves_svg_and_applies_size_stroke_and_color() {
    let def = custom_icon_def("address-book").unwrap();
    let svg = render_custom_icon(def, 18, IconStroke::Sm, "#123456");
    assert!(svg.contains("width=\"18\""), "{svg}");
    assert!(svg.contains("height=\"18\""), "{svg}");
    assert!(svg.contains("stroke-width=\"1.6\""), "{svg}");
    assert!(svg.contains("#123456"), "{svg}");
    assert!(svg.contains("<path"), "{svg}");
}

#[test]
fn custom_lock_keeps_its_own_name_and_tabler_lock_stays_separate() {
    let custom = view! { <IconLockCustom size=16 /> }.into_view().to_html();
    let tabler = view! { <IconLock size=16 /> }.into_view().to_html();
    let def = custom_icon_def("lock").unwrap();
    let first_path = def
        .svg
        .split_once("<path")
        .unwrap()
        .1
        .split_once('>')
        .unwrap()
        .0;
    assert!(custom.contains(&format!("<path{first_path}>")), "{custom}");
    assert!(custom.contains("data-twenty-icon"), "{custom}");
    assert!(tabler.contains("tabler-icon tabler-icon-lock"), "{tabler}");
    assert!(!tabler.contains(&format!("<path{first_path}>")), "{tabler}");
}

#[test]
fn horizontal_chart_keeps_the_upstream_quarter_turn() {
    let html = view! { <IconChartBarHorizontal size=16 /> }
        .into_view()
        .to_html();
    assert!(html.contains("rotate(90deg)"), "{html}");
    assert!(html.contains("tabler-icon-chart-bar"), "{html}");
    assert!(html.contains("stroke-width=\"1.6\""), "{html}");
    assert!(html.trim_start().starts_with("<div"), "{html}");
}

#[test]
fn generic_icon_defaults_to_tabler_size_and_forwards_visual_props() {
    let html = view! {
        <Icon
            name="user"
            class_name="extra-class"
            style="opacity:0.5"
            aria_hidden=true
        />
    }
    .into_view()
    .to_html();
    assert!(html.contains("width=\"24\""), "{html}");
    assert!(html.contains("height=\"24\""), "{html}");
    assert!(
        html.contains("tabler-icon tabler-icon-user  extra-class"),
        "{html}"
    );
    assert!(html.contains("style=\"opacity:0.5;\""), "{html}");
    assert!(html.contains("aria-hidden=\"true\""), "{html}");
    assert!(!html.contains("data-twenty-icon"), "{html}");
}

#[test]
fn optional_aria_hidden_distinguishes_false_from_omitted() {
    let hidden = view! { <Icon name="user" aria_hidden=false /> }
        .into_view()
        .to_html();
    let omitted = view! { <Icon name="user" /> }.into_view().to_html();
    assert!(hidden.contains("aria-hidden=\"false\""), "{hidden}");
    assert!(!omitted.contains("aria-hidden"), "{omitted}");
}

#[test]
fn custom_address_book_keeps_the_svgr_data_attribute() {
    let html = view! { <Icon name="address-book" /> }.into_view().to_html();
    assert!(html.contains("data-twenty-icon=\"\""), "{html}");
    assert!(html.contains("icon-tabler-address-book"), "{html}");
}

#[test]
fn custom_icon_defaults_follow_the_upstream_theme_tokens() {
    let brand = view! { <IconBrandAnthropic /> }.into_view().to_html();
    let address_book = view! { <IconAddressBook /> }.into_view().to_html();
    let lock = view! { <IconLockCustom /> }.into_view().to_html();
    let star = view! { <IconTwentyStar /> }.into_view().to_html();
    assert!(brand.contains("width=\"20\""), "{brand}");
    assert!(address_book.contains("width=\"24\""), "{address_book}");
    assert!(lock.contains("width=\"20\""), "{lock}");
    assert!(star.contains("width=\"24\""), "{star}");
    assert!(star.contains("stroke-width=\"2\""), "{star}");
    assert_eq!(CUSTOM_ICON_CATALOG_TAG, "twenty-v2.61.0-4bac2940dcdc");
}

#[test]
fn illustration_icon_uses_the_upstream_wrapper_and_theme_colors() {
    let html = view! { <IllustrationIconUser /> }.into_view().to_html();
    assert!(
        html.contains("class=\"t-illustration-icon-wrapper\""),
        "{html}"
    );
    assert!(
        html.contains(
            "background-color:var(--t-background-primary);border:0.75px solid var(--t-border-color-medium);border-radius:var(--t-border-radius-md);display:flex;flex-shrink:0;justify-content:center"
        ),
        "{html}"
    );
    assert!(html.contains("width=\"20\""), "{html}");
    assert!(html.contains("fill=\"var(--t-accent-accent3)\""), "{html}");
    assert!(html.contains("currentFill"), "{html}");
    assert!(html.contains("color=\"var(--t-accent-accent8)\""), "{html}");
    assert!(html.contains("data-twenty-icon=\"\""), "{html}");
}

#[test]
fn sparkle_two_forwards_class_style_and_accessibility_props() {
    let html = view! {
        <IconSparkle2
            class_name="extra-class"
            style="opacity:0.5"
            aria_hidden=true
        />
    }
    .into_view()
    .to_html();
    assert!(
        html.contains("tabler-icon tabler-icon-sparkle-2 extra-class"),
        "{html}"
    );
    assert!(html.contains("style=\"opacity:0.5;\""), "{html}");
    assert!(html.contains("aria-hidden=\"true\""), "{html}");
}

#[test]
fn thinking_orbit_loader_keeps_the_upstream_name_and_animation() {
    let html = view! { <ThinkingOrbitLoaderIcon size=14 /> }
        .into_view()
        .to_html();
    assert!(html.contains("values=\"0;-100\""), "{html}");
    assert!(html.contains("dur=\"1.05s\""), "{html}");
}

#[test]
fn icon_dictionary_maps_match_upstream_tabler_names() {
    let dictionary = format!(
        "{}/fixtures/TwentyIconDictionary.ts",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&dictionary)
        .unwrap_or_else(|e| panic!("cannot read {dictionary}: {e}"));
    for (rust_name, tabler_name) in [
        ("IconUser", "user"),
        ("IconBuildingSkyscraper", "building-skyscraper"),
        ("IconTable", "table"),
        ("IconLayoutList", "layout-list"),
        ("IconLayoutKanban", "layout-kanban"),
        ("IconFilter", "filter"),
        ("IconSettings", "settings"),
        ("IconUsers", "users"),
        ("IconLock", "lock"),
        ("IconArrowsSort", "arrows-sort"),
        ("IconSparkles", "sparkles"),
        ("IconKey", "key"),
        ("IconTerminal", "terminal"),
    ] {
        assert!(
            text.contains(&format!("iconName: '{rust_name}'")),
            "{UPSTREAM_TAG}: {rust_name} absent from TwentyIconDictionary"
        );
        assert!(
            text.contains(&format!("tablerName: '{tabler_name}'")),
            "{UPSTREAM_TAG}: {tabler_name} absent from TwentyIconDictionary"
        );
        let def = icon_def(tabler_name).unwrap_or_else(|| panic!("missing port of {tabler_name}"));
        assert_eq!(def.rust_name, rust_name);
    }
}

#[test]
fn tabler_version_is_pinned_where_upstream_declared_it() {
    let lock = format!(
        "{}/fixtures/yarn-lock-tabler.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(&lock).unwrap_or_else(|e| panic!("cannot read {lock}: {e}"));
    assert!(text.contains("\"@tabler/icons@npm:3.41.1\""));
    assert!(text.contains("version: 3.41.1"));
}

#[test]
fn stroke_scale_matches_icon_size_tokens() {
    assert_eq!(IconStroke::Sm.value(), "1.6");
    assert_eq!(IconStroke::Md.value(), "2");
    assert_eq!(IconStroke::Lg.value(), "2.5");
    assert_eq!(
        crate::tokens::token(crate::tokens::Scheme::Light, "--t-icon-size-md"),
        Some(" 16")
    );
    assert_eq!(
        crate::tokens::token(crate::tokens::Scheme::Light, "--t-icon-stroke-sm"),
        Some(" 1.6")
    );
}
