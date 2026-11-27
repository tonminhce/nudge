import re
import sys

fixture_dir, out_path = sys.argv[1], sys.argv[2]

DECL_RE = re.compile(r"^    (--[a-z0-9_-]+):(.*)$")
SUPPORTS_RE = re.compile(r"@supports \(corner-shape: squircle\) \{\n    \.(light|dark) \{\n(.*?)\n    \}", re.S)
CORNER_SHAPE = re.compile(r"^      (--[a-z0-9_-]+):(.+);$")


def parse_scheme(lines):
    decls = []
    i = 0
    while i < len(lines):
        match = DECL_RE.match(lines[i])
        if not match:
            i += 1
            continue
        key = match.group(1)
        rest = match.group(2)
        if rest.rstrip().endswith(";"):
            decls.append((key, rest[:-1]))
        else:
            value_lines = [rest]
            i += 1
            while i < len(lines):
                value_lines.append(lines[i])
                if lines[i].rstrip().endswith(";"):
                    break
                i += 1
            joined = "\n".join(value_lines)
            decls.append((key, joined[:-1]))
        i += 1
    return decls


def parse_squircle(text):
    blocks = {}
    for match in SUPPORTS_RE.finditer(text):
        scheme = match.group(1)
        rules = []
        for line in match.group(2).splitlines():
            decl = CORNER_SHAPE.match(line)
            if decl:
                rules.append((decl.group(1), decl.group(2)))
        blocks[scheme] = rules
    return blocks


def rust_str(s):
    out = ['"']
    for ch in s:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        elif ch == "\r":
            out.append("\\r")
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


def emit_table(name, decls):
    lines = [f"pub const {name}: &[(&str, &str)] = &["]
    for key, value in decls:
        lines.append(f"    ({rust_str(key)}, {rust_str(value)}),")
    lines.append("];")
    return "\n".join(lines)


def emit_squircle(name, rules):
    lines = [f"pub const {name}: &[(&str, &str)] = &["]
    for key, value in rules:
        lines.append(f"    ({rust_str(key)}, {rust_str(value)}),")
    lines.append("];")
    return "\n".join(lines)


with open(f"{fixture_dir}/theme-light.css", encoding="utf-8") as fh:
    light_text = fh.read()
with open(f"{fixture_dir}/theme-dark.css", encoding="utf-8") as fh:
    dark_text = fh.read()

light = parse_scheme(light_text.split("\n"))
dark = parse_scheme(dark_text.split("\n"))
light_sq = parse_squircle(light_text)["light"]
dark_sq = parse_squircle(dark_text)["dark"]

light_names = [k for k, _ in light]
dark_names = [k for k, _ in dark]
assert len(light_names) == len(set(light_names)), "duplicate light tokens"
assert len(dark_names) == len(set(dark_names)), "duplicate dark tokens"
print(f"light: {len(light)} declarations, dark: {len(dark)} declarations")
print(f"squircle: light {len(light_sq)} rules, dark {len(dark_sq)} rules")

parts = [
    "pub const TOKEN_COUNT_LIGHT: usize = " + str(len(light)) + ";",
    "pub const TOKEN_COUNT_DARK: usize = " + str(len(dark)) + ";",
    "pub const TOKEN_COUNT_COLOR: usize = "
    + str(sum(1 for k, _ in light if k.startswith("--t-color-"))) + ";",
    "pub const TOKEN_COUNT_SPACING: usize = "
    + str(sum(1 for k, _ in light if k.startswith("--t-spacing-"))) + ";",
    "pub const TOKEN_COUNT_HUES: usize = 25;",
    "pub const TOKEN_STEPS_PER_HUE: usize = 13;",
    "",
    emit_table("LIGHT_TOKENS", light),
    "",
    emit_table("DARK_TOKENS", dark),
    "",
    emit_squircle("LIGHT_SQUIRCLE", light_sq),
    "",
    emit_squircle("DARK_SQUIRCLE", dark_sq),
]
with open(out_path, "w", encoding="utf-8") as fh:
    fh.write("\n".join(parts) + "\n")
print("wrote", out_path)
