import json
import os
import re
import subprocess
import sys

ASSETS = {
    "address-book": "IconAddressBook",
    "anthropic": "IconBrandAnthropic",
    "claude": "IconModelClaude",
    "eu-stars": "IconEuStars",
    "gemini": "IconBrandGemini",
    "gmail": "IconGmail",
    "google-calendar": "IconGoogleCalendar",
    "google": "IconGoogle",
    "groq": "IconBrandGroq",
    "hubspot": "IconBrandHubspot",
    "illustration-array": "IllustrationIconArray",
    "illustration-calendar-event": "IllustrationIconCalendarEvent",
    "illustration-calendar-time": "IllustrationIconCalendarTime",
    "illustration-currency": "IllustrationIconCurrency",
    "illustration-file": "IllustrationIconFile",
    "illustration-json": "IllustrationIconJson",
    "illustration-link": "IllustrationIconLink",
    "illustration-mail": "IllustrationIconMail",
    "illustration-many-to-many": "IllustrationIconManyToMany",
    "illustration-map": "IllustrationIconMap",
    "illustration-numbers": "IllustrationIconNumbers",
    "illustration-one-to-many": "IllustrationIconOneToMany",
    "illustration-one-to-one": "IllustrationIconOneToOne",
    "illustration-phone": "IllustrationIconPhone",
    "illustration-setting": "IllustrationIconSetting",
    "illustration-star": "IllustrationIconStar",
    "illustration-tag": "IllustrationIconTag",
    "illustration-tags": "IllustrationIconTags",
    "illustration-text": "IllustrationIconText",
    "illustration-toggle": "IllustrationIconToggle",
    "illustration-uid": "IllustrationIconUid",
    "illustration-user": "IllustrationIconUser",
    "lock": "IconLockCustom",
    "many-to-one": "IconRelationManyToOne",
    "microsoft-calendar": "IconMicrosoftCalendar",
    "microsoft-outlook": "IconMicrosoftOutlook",
    "microsoft": "IconMicrosoft",
    "mistral": "IconBrandMistral",
    "openai": "IconProviderOpenai",
    "pipedrive": "IconBrandPipedrive",
    "salesforce": "IconBrandSalesforce",
    "trash-x-off": "IconTrashXOff",
    "twenty-star": "IconTwentyStar",
    "twenty-star-filled": "IconTwentyStarFilled",
    "typesafe-ai": "IconBrandTypesafeAi",
    "xai": "IconBrandXai",
}

REF = os.environ.get(
    "TWENTY_REFERENCE_REF", "4bac2940dcdcbf4bb4c974b628cd5e1b0a2e37e0"
)
API_ROOT = "repos/twentyhq/twenty/contents/packages/twenty-ui/src/assets/icons"
STROKE_ICONS = {
    "address-book",
    "many-to-one",
    "trash-x-off",
    "twenty-star",
    "twenty-star-filled",
}
COLOR_MODES = {
    "address-book": "Stroke",
    "openai": "Root",
    "typesafe-ai": "Root",
    "xai": "Root",
}
DEFAULT_SIZE_24 = {
    "address-book",
    "many-to-one",
    "twenty-star",
    "twenty-star-filled",
}


def rust_str(value):
    out = ['"']
    for character in value:
        if character == "\\":
            out.append("\\\\")
        elif character == '"':
            out.append('\\"')
        elif character == "\n":
            out.append("\\n")
        elif character == "\r":
            out.append("\\r")
        elif character == "\t":
            out.append("\\t")
        else:
            out.append(character)
    out.append('"')
    return "".join(out)


def attr(svg, name, default=None):
    match = re.search(rf'\b{name}="([^"]+)"', svg)
    return match.group(1) if match else default


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: gen_custom_icons.py OUTPUT_RS")

    listing = json.loads(
        subprocess.check_output(["gh", "api", f"{API_ROOT}?ref={REF}"], text=True)
    )
    upstream_assets = {
        entry["name"][:-4]
        for entry in listing
        if entry["type"] == "file" and entry["name"].endswith(".svg")
    }
    if upstream_assets != set(ASSETS):
        missing = sorted(upstream_assets - set(ASSETS))
        extra = sorted(set(ASSETS) - upstream_assets)
        raise SystemExit(f"custom icon asset set drifted: missing={missing}, extra={extra}")

    entries = []
    for name, rust_name in ASSETS.items():
        source = subprocess.check_output(
            [
                "gh",
                "api",
                "-H",
                "Accept: application/vnd.github.raw+json",
                f"{API_ROOT}/{name}.svg?ref={REF}",
            ],
            text=True,
        )
        source = re.sub(r"<\?xml[^>]*\?>", "", source)
        source = re.sub(r"<!--[\s\S]*?-->", "", source).strip()
        root = re.search(r"<svg\b[^>]*>", source)
        if not root:
            raise SystemExit(f"{name}: missing svg root")
        width = attr(root.group(0), "width", "24")
        height = attr(root.group(0), "height", "24")
        stroke_width = attr(root.group(0), "stroke-width")
        normalized = root.group(0)
        if not re.search(r'\bwidth="', normalized):
            normalized = normalized[:-1] + f' width="{width}">'
        if not re.search(r'\bheight="', normalized):
            normalized = normalized[:-1] + f' height="{height}">'
        if not re.search(r'\bdata-twenty-icon="', normalized):
            normalized = normalized[:-1] + ' data-twenty-icon="">'
        source = normalized + source[root.end():]
        entries.append(
            {
                "name": name,
                "rust_name": rust_name,
                "svg": source,
                "width": width,
                "height": height,
                "stroke_width": stroke_width,
                "default_size": 24 if name in DEFAULT_SIZE_24 else 20,
                "supports_stroke": name in STROKE_ICONS,
                "color_mode": COLOR_MODES.get(name, "None"),
                "illustration": name.startswith("illustration-"),
            }
        )
    entries.sort(key=lambda entry: entry["name"])

    lines = [
        "use leptos::prelude::*;",
        "",
        "use super::IconStroke;",
        "",
        "#[derive(Clone, Copy, PartialEq, Eq)]",
        "pub enum CustomIconColorMode {",
        "    None,",
        "    Stroke,",
        "    Root,",
        "}",
        "",
        "pub struct CustomIconDef {",
        "    pub name: &'static str,",
        "    pub rust_name: &'static str,",
        "    pub svg: &'static str,",
        "    pub width: &'static str,",
        "    pub height: &'static str,",
        "    pub stroke_width: Option<&'static str>,",
        "    pub default_size: u16,",
        "    pub supports_stroke: bool,",
        "    pub color_mode: CustomIconColorMode,",
        "    pub illustration: bool,",
        "}",
        "",
        f"pub const CUSTOM_ICON_CATALOG_TAG: &str = {rust_str('twenty-v2.61.0-' + REF[:12])};",
        "",
        "pub static CUSTOM_ICON_DEFS: &[CustomIconDef] = &[",
    ]
    for entry in entries:
        stroke_width = (
            f"Some({rust_str(entry['stroke_width'])})"
            if entry["stroke_width"] is not None
            else "None"
        )
        lines.extend(
            [
                "    CustomIconDef {",
                f"        name: {rust_str(entry['name'])},",
                f"        rust_name: {rust_str(entry['rust_name'])},",
                f"        svg: {rust_str(entry['svg'])},",
                f"        width: {rust_str(entry['width'])},",
                f"        height: {rust_str(entry['height'])},",
                f"        stroke_width: {stroke_width},",
                f"        default_size: {entry['default_size']},",
                f"        supports_stroke: {str(entry['supports_stroke']).lower()},",
                f"        color_mode: CustomIconColorMode::{entry['color_mode']},",
                f"        illustration: {str(entry['illustration']).lower()},",
                "    },",
            ]
        )
    lines.extend(
        [
            "];",
            "",
            f"pub const CUSTOM_ICON_COUNT: usize = {len(entries)};",
            "",
            "pub fn custom_icon_def(name: &str) -> Option<&'static CustomIconDef> {",
            "    CUSTOM_ICON_DEFS",
            "        .binary_search_by(|def| def.name.cmp(name))",
            "        .ok()",
            "        .map(|index| &CUSTOM_ICON_DEFS[index])",
            "}",
            "",
            "fn escape_attribute(value: &str) -> String {",
            "    value",
            "        .replace('&', \"&amp;\")",
            "        .replace('<', \"&lt;\")",
            "        .replace('>', \"&gt;\")",
            "        .replace('\"', \"&quot;\")",
            "}",
            "",
            "fn set_root_attribute(svg: &str, name: &str, value: &str) -> String {",
            "    let Some(start) = svg.find(\"<svg\") else {",
            "        return svg.to_string();",
            "    };",
            "    let Some(relative_end) = svg[start..].find('>') else {",
            "        return svg.to_string();",
            "    };",
            "    let end = start + relative_end;",
            "    let root = &svg[start..end];",
            "    let attribute = format!(\" {name}=\\\"\");",
            "    if let Some(attribute_start) = root.find(&attribute) {",
            "        let value_start = start + attribute_start + attribute.len();",
            "        if let Some(relative_value_end) = svg[value_start..end].find('\"') {",
            "            let value_end = value_start + relative_value_end;",
            "            let mut updated = svg.to_string();",
            "            updated.replace_range(value_start..value_end, value);",
            "            return updated;",
            "        }",
            "    }",
            "    let mut updated = svg.to_string();",
            "    updated.insert_str(end, &format!(\" {name}=\\\"{value}\\\"\"));",
            "    updated",
            "}",
            "",
            "pub fn render_custom_icon(",
            "    def: &CustomIconDef,",
            "    size: u16,",
            "    stroke: IconStroke,",
            "    color: &str,",
            ") -> String {",
            "    let size = size.to_string();",
            "    let color = escape_attribute(color);",
            "    let mut svg = def.svg.to_string();",
            "    svg = svg.replacen(",
            "        &format!(\" width=\\\"{}\\\"\", def.width),",
            "        &format!(\" width=\\\"{size}\\\"\"),",
            "        1,",
            "    );",
            "    svg = svg.replacen(",
            "        &format!(\" height=\\\"{}\\\"\", def.height),",
            "        &format!(\" height=\\\"{size}\\\"\"),",
            "        1,",
            "    );",
            "    match def.color_mode {",
            "        CustomIconColorMode::None => {}",
            "        CustomIconColorMode::Stroke => {",
            "            svg = svg.replacen(",
            "                \"stroke=\\\"currentColor\\\"\",",
            "                &format!(\"stroke=\\\"{color}\\\"\"),",
            "                1,",
            "            );",
            "        }",
            "        CustomIconColorMode::Root => {",
            "            svg = set_root_attribute(&svg, \"color\", &color);",
            "        }",
            "    }",
            "    if def.illustration {",
            "        svg = set_root_attribute(&svg, \"fill\", \"var(--t-accent-accent3)\");",
            "        svg = set_root_attribute(&svg, \"color\", \"var(--t-accent-accent8)\");",
            "    }",
            "    if def.supports_stroke {",
            "        if let Some(original) = def.stroke_width {",
            "            svg = svg.replacen(",
            "                &format!(\" stroke-width=\\\"{original}\\\"\"),",
            "                &format!(\" stroke-width=\\\"{}\\\"\", stroke.value()),",
            "                1,",
            "            );",
            "        } else {",
            "            svg = set_root_attribute(&svg, \"stroke-width\", stroke.value());",
            "        }",
            "    }",
            "    svg",
            "}",
            "",
            "#[component]",
            "pub fn CustomIcon(",
            "    name: &'static str,",
            "    #[prop(optional)] size: u16,",
            "    #[prop(optional)] stroke: Option<IconStroke>,",
            "    #[prop(optional)] color: &'static str,",
            ") -> impl IntoView {",
            "    let def = custom_icon_def(name).unwrap_or(&CUSTOM_ICON_DEFS[0]);",
            "    let size = if size == 0 { def.default_size } else { size };",
            "    let stroke = stroke.unwrap_or(IconStroke::Md);",
            "    let color = if color.is_empty() { \"currentColor\" } else { color };",
            "    let svg = render_custom_icon(def, size, stroke, color);",
            "    let svg_view = leptos::tachys::html::InertElement::new(svg);",
            "    if def.illustration {",
            "        view! {",
            "            <div",
            "                class=\"t-illustration-icon-wrapper\"",
            "                style=\"background-color:var(--t-background-primary);border:0.75px solid var(--t-border-color-medium);border-radius:var(--t-border-radius-md);display:flex;flex-shrink:0;justify-content:center\"",
            "            >",
            "                {svg_view}",
            "            </div>",
            "        }",
            "        .into_any()",
            "    } else {",
            "        svg_view.into_any()",
            "    }",
            "}",
            "",
        ]
    )
    for entry in entries:
        lines.extend(
            [
                "#[component]",
                f"pub fn {entry['rust_name']}(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {{",
                f"    view! {{ <CustomIcon name={rust_str(entry['name'])} size=size stroke=stroke.unwrap_or_default() color=color/> }}",
                "}",
                "",
            ]
        )
    with open(sys.argv[1], "w", encoding="utf-8") as output:
        output.write("\n".join(lines).rstrip() + "\n")
    print(f"wrote {len(entries)} custom icons from {REF[:12]} to {sys.argv[1]}")


if __name__ == "__main__":
    main()
