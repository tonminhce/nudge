import re
import sys

svg_dir, out_path = sys.argv[1], sys.argv[2]

ICON_NAMES = [
    "user", "users", "mail", "lock", "eye", "eye-off", "check", "x",
    "chevron-down", "chevron-up", "chevron-left", "chevron-right",
    "arrow-right", "sun", "moon", "settings", "building", "building-skyscraper", "home",
    "search", "plus", "sparkles", "table", "layout-list", "layout-kanban",
    "calendar", "filter", "arrows-sort", "selector", "dots", "dots-vertical",
    "pencil", "trash", "star", "bookmark", "menu-2", "refresh",
    "alert-triangle", "info-circle", "columns-3", "grip-vertical", "tag",
    "phone", "external-link", "copy", "key", "terminal", "world",
    "chevrons-left", "loader", "bell",
]

PATH_LINE_RE = re.compile(r"^  (<path .*/>)$")
VIEWBOX_RE = re.compile(r'viewBox="([^"]+)"')


def pascal(name):
    return "".join(part.capitalize() for part in name.split("-"))


def rust_str(s):
    out = ['"']
    for ch in s:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


entries = []
for name in ICON_NAMES:
    with open(f"{svg_dir}/{name}.svg", encoding="utf-8") as fh:
        src = fh.read()
    view_box = VIEWBOX_RE.search(src).group(1)
    raw_lines = []
    for line in src.split("\n"):
        match = PATH_LINE_RE.match(line)
        if match:
            raw_lines.append(match.group(1))
    assert raw_lines, f"{name}: no path lines"
    d_values = []
    for raw in raw_lines:
        dm = re.search(r'd="([^"]+)"', raw)
        assert dm, f"{name}: path without d: {raw}"
        d_values.append(dm.group(1))
    assert len(raw_lines) == len(d_values)
    entries.append(
        {
            "rust": "Icon" + pascal(name),
            "tabler": name,
            "view_box": view_box,
            "raws": raw_lines,
            "ds": d_values,
        }
    )

lines = [
    "pub struct IconDef {",
    "    pub rust_name: &'static str,",
    "    pub tabler_name: &'static str,",
    "    pub view_box: &'static str,",
    "    pub raw_paths: &'static [&'static str],",
    "    pub d_values: &'static [&'static str],",
    "}",
    "",
    "pub const ICON_DEFS: &[IconDef] = &[",
]
for e in entries:
    lines.append("    IconDef {")
    lines.append(f"        rust_name: {rust_str(e['rust'])},")
    lines.append(f"        tabler_name: {rust_str(e['tabler'])},")
    lines.append(f"        view_box: {rust_str(e['view_box'])},")
    lines.append("        raw_paths: &[" + ", ".join(rust_str(r) for r in e["raws"]) + "],")
    lines.append("        d_values: &[" + ", ".join(rust_str(d) for d in e["ds"]) + "],")
    lines.append("    },")
lines.append("];")
lines.append("")
lines.append(f"pub const ICON_COUNT: usize = {len(entries)};")
lines.append("")
for e in entries:
    rust = e["rust"]
    name = e["tabler"]
    lines.append("#[component]")
    lines.append(
        "pub fn "
        + rust
        + "(#[prop(optional)] size: u16, #[prop(optional)] stroke: Option<IconStroke>, "
        + "#[prop(optional)] color: &'static str) -> impl IntoView {"
    )
    lines.append(
        f"    view! {{ <Icon name=\"{name}\" size=size "
        "stroke=stroke.unwrap_or_default() color=color/> }"
    )
    lines.append("}")
    lines.append("")

with open(out_path, "w", encoding="utf-8") as fh:
    fh.write("\n".join(lines) + "\n")
print(f"wrote {len(entries)} icon defs to {out_path}")
