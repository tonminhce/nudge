import json
import os
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE = os.path.join(REPO_ROOT, "baseline")
DEFINITIONS = os.path.join(
    REPO_ROOT, "crates", "server", "src", "standard_objects", "definitions.rs"
)

ACCESS_COLUMNS = ("_accessGroupIds", "_isPrivate")

COMPOSITE_SUFFIXES = {
    "ACTOR": ["Source", "WorkspaceMemberId", "Name", "Context"],
    "FULL_NAME": ["FirstName", "LastName"],
    "LINKS": ["PrimaryLinkLabel", "PrimaryLinkUrl", "SecondaryLinks"],
    "EMAILS": ["PrimaryEmail", "AdditionalEmails"],
    "PHONES": [
        "PrimaryPhoneNumber",
        "PrimaryPhoneCountryCode",
        "PrimaryPhoneCallingCode",
        "AdditionalPhones",
    ],
    "CURRENCY": ["AmountMicros", "CurrencyCode"],
    "ADDRESS": [
        "AddressStreet1",
        "AddressStreet2",
        "AddressCity",
        "AddressPostcode",
        "AddressState",
        "AddressCountry",
        "AddressLat",
        "AddressLng",
    ],
    "RICH_TEXT": ["Blocknote", "Markdown", "Tiptap"],
}

RELATION_TYPES = ("RELATION", "MORPH_RELATION")


def parse_baseline_tables(sql_path):
    with open(sql_path) as handle:
        text = handle.read()
    tables = {}
    pattern = re.compile(r'CREATE TABLE [\w".]+\.(?:"([^"]+)"|(\w+)) \(\n(.*?)\n\);', re.S)
    for match in pattern.finditer(text):
        table_name = match.group(1) or match.group(2)
        columns = []
        logical = ""
        depth = 0
        for raw_line in match.group(3).split("\n"):
            logical += raw_line.strip() + " "
            depth += raw_line.count("(") - raw_line.count(")")
            if depth > 0:
                continue
            stripped = logical.strip().rstrip(",")
            logical = ""
            if not stripped:
                continue
            upper = stripped.upper()
            if upper.startswith(
                ("CONSTRAINT", "PRIMARY KEY", "UNIQUE", "FOREIGN KEY", "CHECK")
            ):
                continue
            if stripped.startswith('"'):
                columns.append(stripped[1 : stripped.index('"', 1)])
            else:
                columns.append(re.match(r"[A-Za-z_]\w*", stripped).group(0))
        tables[table_name] = columns
    return tables


def _unescape(value):
    return value.replace('\\"', '"').replace("\\\\", "\\")


def _rust_string_field(body, key):
    match = re.search(rf'^\s*{key}: Some\("((?:[^"\\]|\\.)*)"\)', body, re.M)
    if match:
        return _unescape(match.group(1))
    if re.search(rf"^\s*{key}: None", body, re.M):
        return None
    match = re.search(rf'^\s*{key}: "((?:[^"\\]|\\.)*)"', body, re.M)
    if match:
        return _unescape(match.group(1))
    return None


def parse_definitions_fields(definitions_path):
    with open(definitions_path) as handle:
        text = handle.read()
    arrays = {}
    array_pattern = re.compile(
        r"pub const (OBJECT_FIELDS_\w+): &\[StandardFieldDefinition\] = &\[(.*?)\n\];",
        re.S,
    )
    for match in array_pattern.finditer(text):
        suffix = match.group(1)
        entries = []
        for entry_match in re.finditer(
            r"StandardFieldDefinition \{(.*?)\n    \},", match.group(2), re.S
        ):
            body = entry_match.group(1)
            name = _rust_string_field(body, "name")
            field_type = _rust_string_field(body, "field_type")
            morph_id = _rust_string_field(body, "morph_id")
            universal_id = _rust_string_field(body, "universal_identifier")
            settings_json = _rust_string_field(body, "settings_json")
            settings = json.loads(settings_json) if settings_json else {}
            entries.append(
                {
                    "name": name,
                    "type": field_type,
                    "morphId": morph_id,
                    "universalIdentifier": universal_id,
                    "settings": settings,
                }
            )
        arrays[suffix] = entries
    objects = []
    for match in re.finditer(
        r"StandardObjectDefinition \{(.*?)\n    \},", text, re.S
    ):
        body = match.group(1)
        if "pub name_singular" in body:
            continue
        name_singular = _rust_string_field(body, "name_singular")
        fields_ref = re.search(r"fields: (OBJECT_FIELDS_\w+),", body)
        if not (name_singular and fields_ref):
            continue
        objects.append(
            {
                "name_singular": name_singular,
                "fields_ref": fields_ref.group(1),
                "fields": arrays.get(fields_ref.group(1), []),
            }
        )
    return objects


def field_columns(field):
    field_type = field["type"]
    settings = field.get("settings") or {}
    if field_type == "RELATION":
        if settings.get("relationType") == "MANY_TO_ONE":
            return [settings.get("joinColumnName") or f"{field['name']}Id"]
        return []
    if field_type == "MORPH_RELATION":
        if field.get("morphId") == field.get("universalIdentifier"):
            return []
        return [settings.get("joinColumnName") or f"{field['name']}Id"]
    if field_type in COMPOSITE_SUFFIXES:
        return [field["name"] + suffix for suffix in COMPOSITE_SUFFIXES[field_type]]
    return [field["name"]]


def access_bounds(columns):
    present = [columns.index(c) for c in ACCESS_COLUMNS if c in columns]
    if not present:
        return len(columns), len(columns) - 1
    return min(present), max(present)


def verify(tables, objects):
    results = []
    for object_row in sorted(objects, key=lambda o: o["name_singular"]):
        table = tables.get(object_row["name_singular"])
        if table is None:
            results.append((object_row["name_singular"], False, "no baseline table"))
            continue
        lo, hi = access_bounds(table)
        head_expected = table[:lo]
        tail_expected = table[hi + 1 :]
        inline_actual = []
        relation_actual = []
        for field in object_row["fields"]:
            columns = field_columns(field)
            if field["type"] in RELATION_TYPES:
                relation_actual.extend(columns)
            else:
                inline_actual.extend(columns)
        problems = []
        if inline_actual != head_expected:
            problems.append(_diff("inline", head_expected, inline_actual))
        if relation_actual != tail_expected:
            problems.append(_diff("relation", tail_expected, relation_actual))
        if problems:
            results.append(
                (object_row["name_singular"], False, " | ".join(problems)[:120])
            )
        else:
            results.append(
                (
                    object_row["name_singular"],
                    True,
                    f"inline={len(inline_actual)} relation={len(relation_actual)}",
                )
            )
    return results


def _diff(kind, expected, actual):
    return f"{kind} order mismatch (expected {len(expected)} got {len(actual)})"


def main():
    tables = parse_baseline_tables(
        os.path.join(BASELINE, "workspace-schema-2.61.0.sql")
    )
    objects = parse_definitions_fields(DEFINITIONS)
    results = verify(tables, objects)
    width = max(len(name) for name, _, _ in results)
    print(f"{'TABLE'.ljust(width)}  STATUS  DETAIL")
    passed = 0
    for name, status, detail in results:
        flag = "PASS" if status else "FAIL"
        if status:
            passed += 1
        print(f"{name.ljust(width)}  {flag}    {detail}")
    total = len(results)
    print(f"\n{passed}/{total} tables ordered")
    return 0 if passed == total == 37 else 1


if __name__ == "__main__":
    sys.exit(main())
