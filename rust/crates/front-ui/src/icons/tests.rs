use crate::icons::{
    icon_class, icon_def, object_icon, render_icon, IconStroke, ICON_COUNT, ICON_DEFS, OBJECT_ICONS,
};
use crate::UPSTREAM_TAG;

fn fixture(name: &str) -> String {
    let path = format!("{}/fixtures/icons/{name}.svg", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
}

fn path_lines(svg: &str) -> Vec<String> {
    svg.lines()
        .filter_map(|line| {
            let trimmed = line
                .strip_prefix("  ")
                .and_then(|l| l.strip_suffix(" />"))
                .and_then(|l| l.strip_prefix("<path "));
            trimmed.map(|attrs| format!("<path {attrs} />"))
        })
        .collect()
}

#[test]
fn icon_count_matches_expected_set() {
    assert_eq!(ICON_COUNT, 56);
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
fn every_icon_path_data_equals_the_upstream_svg_fixture() {
    for def in ICON_DEFS {
        let svg = fixture(def.tabler_name);
        let fixture_lines = path_lines(&svg);
        assert!(
            !fixture_lines.is_empty(),
            "{}: no path lines parsed from fixture",
            def.tabler_name
        );
        assert_eq!(
            fixture_lines.len(),
            def.raw_paths.len(),
            "{}: path count differs from fixture",
            def.tabler_name
        );
        for (want, got) in fixture_lines.iter().zip(def.raw_paths.iter()) {
            assert_eq!(want, got, "{}: path element bytes differ", def.tabler_name);
        }
        assert!(
            svg.contains(&format!("viewBox=\"{}\"", def.view_box)),
            "{}: view box not in fixture",
            def.tabler_name
        );
    }
}

#[test]
fn d_values_are_the_exact_path_d_attributes_from_the_fixture() {
    for def in ICON_DEFS {
        let svg = fixture(def.tabler_name);
        let fixture_ds = svg
            .lines()
            .filter_map(|line| {
                let idx = line.find("d=\"")?;
                let rest = &line[idx + 3..];
                let end = rest.find('"')?;
                Some(rest[..end].to_string())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            fixture_ds,
            def.d_values
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            "{}: d attribute list differs",
            def.tabler_name
        );
    }
}

#[test]
fn rendered_svg_uses_the_upstream_root_attributes_and_class() {
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
    assert!(
        html.contains("tabler-icon-user"),
        "{html}: icon class must carry the tabler kebab name"
    );
}

#[test]
fn rendered_svg_carries_every_d_value_as_a_path() {
    let def = icon_def("calendar").expect("calendar present");
    let html = render_icon(def, 24, IconStroke::Md, "currentColor");
    for d in def.d_values {
        assert!(
            html.contains(&format!("<path d=\"{d}\"/>")),
            "{}: missing path d in render",
            def.tabler_name
        );
    }
    assert_eq!(html.matches("<path").count(), def.d_values.len());
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
