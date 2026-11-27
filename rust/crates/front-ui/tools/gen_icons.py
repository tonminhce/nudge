import json
import os
import re
import subprocess
import sys

WRAPPER_NAMES = [
    "user", "users", "mail", "eye", "eye-off", "check", "x",
    "chevron-down", "chevron-up", "chevron-left", "chevron-right",
    "arrow-right", "sun", "moon", "settings", "building", "building-skyscraper", "home",
    "search", "plus", "sparkles", "table", "layout-list", "layout-kanban",
    "calendar", "filter", "arrows-sort", "selector", "dots", "dots-vertical",
    "pencil", "trash", "star", "bookmark", "menu-2", "refresh",
    "alert-triangle", "info-circle", "columns-3", "grip-vertical", "tag",
    "phone", "external-link", "copy", "key", "terminal", "world",
    "chevrons-left", "loader", "bell", "target-arrow", "checkbox", "notes",
    "settings-automation", "lego", "chart-bar",
]

NODE_SCRIPT = r"""
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { t as createBundledReact } from '/app/packages/twenty-server/dist/front/assets/react-DlfctsU9.js';
import { ALL_ICONS } from '/app/packages/twenty-server/dist/front/assets/AllIcons-DbdVWsOq-BaGDDuxt.js';
import { a as ThemeProvider } from '/app/packages/twenty-server/dist/front/assets/theme-B1jMC8En-CqsfDy53.js';
const BundledReact = createBundledReact();
for (const hook of ['useContext', 'useState', 'useRef', 'useMemo', 'useLayoutEffect', 'useEffect', 'useCallback', 'useDebugValue', 'useId']) {
  BundledReact[hook] = React[hook];
}
const catalog = Object.entries(ALL_ICONS).map(([rust_name, Component]) => {
  const html = renderToStaticMarkup(React.createElement(ThemeProvider, {
    colorScheme: 'light',
    applyToRoot: false,
  }, React.createElement(Component, {
    size: 24,
    stroke: 2,
    color: 'currentColor',
  })));
  const root = html.match(/<svg\b[^>]*>([\s\S]*)<\/svg>/);
  const svgTag = html.match(/<svg\b[^>]*>/)?.[0] ?? '';
  const viewBox = svgTag.match(/viewBox="([^"]+)"/);
  const tablerName = svgTag.match(/(?:tabler-icon|icon-tabler)-([a-z0-9-]+)/);
  const className = svgTag.match(/class="([^"]+)"/);
  if (!root || !viewBox || !tablerName || !className) throw new Error(`Cannot parse ${rust_name}: ${html}`);
  return {
    rust_name,
    tabler_name: tablerName[1],
    view_box: viewBox[1],
    class_name: className[1],
    inner_svg: root[1],
  };
}).sort((a, b) => a.tabler_name < b.tabler_name ? -1 : a.tabler_name > b.tabler_name ? 1 : 0);
process.stdout.write(JSON.stringify(catalog));
"""


def pascal(name):
    return "".join(part.capitalize() for part in name.split("-"))


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


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: gen_icons.py OUTPUT_RS")
    container = os.environ.get("TWENTY_REFERENCE_CONTAINER", "twenty-ref")
    catalog = json.loads(subprocess.check_output([
        "docker", "exec", container, "node", "--input-type=module", "-e", NODE_SCRIPT,
    ]))
    if len(catalog) != 4187:
        raise SystemExit(f"expected 4187 upstream icons, got {len(catalog)}")
    by_name = {entry["tabler_name"]: entry for entry in catalog}
    missing = [name for name in WRAPPER_NAMES if name not in by_name]
    if missing:
        raise SystemExit(f"missing wrapper icons: {missing}")

    lines = [
        "pub struct IconDef {",
        "    pub rust_name: &'static str,",
        "    pub tabler_name: &'static str,",
        "    pub view_box: &'static str,",
        "    pub class_name: &'static str,",
        "    pub inner_svg: &'static str,",
        "}",
        "",
        "pub const ICON_CATALOG_TAG: &str = \"twenty-v2.61.0-tabler-3.41.1\";",
        "",
        "pub static ICON_DEFS: &[IconDef] = &[",
    ]
    for entry in catalog:
        lines.append("    IconDef {")
        lines.append(f"        rust_name: {rust_str(entry['rust_name'])},")
        lines.append(f"        tabler_name: {rust_str(entry['tabler_name'])},")
        lines.append(f"        view_box: {rust_str(entry['view_box'])},")
        lines.append(f"        class_name: {rust_str(entry['class_name'])},")
        lines.append(f"        inner_svg: {rust_str(entry['inner_svg'])},")
        lines.append("    },")
    lines.extend([
        "];",
        "",
        f"pub const ICON_COUNT: usize = {len(catalog)};",
        "",
    ])
    for name in WRAPPER_NAMES:
        rust_name = "Icon" + pascal(name)
        lines.extend([
            "#[component]",
            f"pub fn {rust_name}(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, #[prop(optional)] color: &'static str) -> impl IntoView {{",
            f"    view! {{ <Icon name={rust_str(name)} size=size stroke=stroke.unwrap_or_default() color=color/> }}",
            "}",
            "",
        ])
    with open(sys.argv[1], "w", encoding="utf-8") as output:
        output.write("\n".join(lines) + "\n")
    print(f"wrote {len(catalog)} upstream icon defs to {sys.argv[1]}")


if __name__ == "__main__":
    main()
