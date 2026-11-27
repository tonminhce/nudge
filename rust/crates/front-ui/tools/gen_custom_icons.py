import json
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
    "lock": "IconLock",
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

API_ROOT = "repos/twentyhq/twenty/contents/packages/twenty-ui/src/assets/icons"


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


def attr(svg, name):
    match = re.search(rf'\b{name}="([^"]+)"', svg)
    return match.group(1) if match else "24"


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: gen_custom_icons.py OUTPUT_RS")
    entries = []
    for name, rust_name in ASSETS.items():
        source = subprocess.check_output([
            "gh", "api", "-H", "Accept: application/vnd.github.raw+json",
            f"{API_ROOT}/{name}.svg?ref=main",
        ], text=True)
        source = re.sub(r"<\?xml[^>]*\?>", "", source)
        source = re.sub(r"<!--[\s\S]*?-->", "", source).strip()
        root = re.search(r"<svg\b[^>]*>", source)
        if not root:
            raise SystemExit(f"{name}: missing svg root")
        width = attr(root.group(0), "width")
        height = attr(root.group(0), "height")
        stroke_width = attr(root.group(0), "stroke-width")
        normalized = root.group(0)
        if not re.search(r'\bwidth="', normalized):
            normalized = normalized[:-1] + f' width="{width}">'
        if not re.search(r'\bheight="', normalized):
            normalized = normalized[:-1] + f' height="{height}">'
        source = normalized + source[root.end():]
        entries.append({
            "name": name,
            "rust_name": rust_name,
            "svg": source,
            "width": width,
            "height": height,
            "stroke_width": stroke_width,
        })
    entries.sort(key=lambda entry: entry["name"])

    lines = [
        "use leptos::prelude::*;",
        "",
        "use super::IconStroke;",
        "",
        "pub struct CustomIconDef {",
        "    pub name: &'static str,",
        "    pub rust_name: &'static str,",
        "    pub svg: &'static str,",
        "    pub width: &'static str,",
        "    pub height: &'static str,",
        "    pub stroke_width: &'static str,",
        "}",
        "",
        "pub static CUSTOM_ICON_DEFS: &[CustomIconDef] = &[",
    ]
    for entry in entries:
        lines.append("    CustomIconDef {")
        lines.append(f"        name: {rust_str(entry['name'])},")
        lines.append(f"        rust_name: {rust_str(entry['rust_name'])},")
        lines.append(f"        svg: {rust_str(entry['svg'])},")
        lines.append(f"        width: {rust_str(entry['width'])},")
        lines.append(f"        height: {rust_str(entry['height'])},")
        lines.append(f"        stroke_width: {rust_str(entry['stroke_width'])},")
        lines.append("    },")
    lines.extend([
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
        "pub fn render_custom_icon(def: &CustomIconDef, size: u16, stroke: IconStroke, color: &str) -> String {",
        "    let size = size.to_string();",
        "    let color = escape_attribute(color);",
        "    let mut svg = def.svg.to_string();",
        "    svg = svg.replacen(&format!(\" width=\\\"{}\\\"\", def.width), &format!(\" width=\\\"{size}\\\"\"), 1);",
        "    svg = svg.replacen(&format!(\" height=\\\"{}\\\"\", def.height), &format!(\" height=\\\"{size}\\\"\"), 1);",
        "    svg = svg.replace(\"currentColor\", &color);",
        "    svg = svg.replacen(&format!(\" stroke-width=\\\"{}\\\"\", def.stroke_width), &format!(\" stroke-width=\\\"{}\\\"\", stroke.value()), 1);",
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
        "    let def = custom_icon_def(name).expect(\"known custom icon\");",
        "    let size = if size == 0 { 16 } else { size };",
        "    let stroke = stroke.unwrap_or_default();",
        "    let color = if color.is_empty() { \"currentColor\" } else { color };",
        "    let svg = render_custom_icon(def, size, stroke, color);",
        "    view! { <span class=\"t-custom-icon\" style=\"display:contents\" inner_html=svg /> }",
        "}",
        "",
    ])
    for entry in entries:
        lines.extend([
            "#[component]",
            f"pub fn {entry['rust_name']}(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {{",
            f"    view! {{ <CustomIcon name={rust_str(entry['name'])} size=size stroke=stroke.unwrap_or_default() color=color/> }}",
            "}",
            "",
        ])
    with open(sys.argv[1], "w", encoding="utf-8") as output:
        output.write("\n".join(lines) + "\n")
    print(f"wrote {len(entries)} custom icons to {sys.argv[1]}")


if __name__ == "__main__":
    main()
