import json
import os
import re

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE = os.path.join(REPO_ROOT, "baseline")
OUTPUT = os.path.join(
    REPO_ROOT, "crates", "server", "src", "standard_objects", "definitions.rs"
)

STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER = "20202020-64aa-4b6f-b003-9c74b97cee20"

COMPOSITE_FIRST_COLUMN_SUFFIXES = {
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

NO_COLUMN_SORT_KEY = 10**9


def parse_baseline_column_orders(sql_path):
    with open(sql_path) as handle:
        text = handle.read()
    orders = {}
    pattern = re.compile(
        r'CREATE TABLE [\w".]+\.(?:"([^"]+)"|(\w+)) \(\n(.*?)\n\);', re.S
    )
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
        orders[table_name] = {name: index for index, name in enumerate(columns)}
    return orders


def field_physical_columns(field):
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
    if field_type in COMPOSITE_FIRST_COLUMN_SUFFIXES:
        return [field["name"] + suffix for suffix in COMPOSITE_FIRST_COLUMN_SUFFIXES[field_type]]
    return [field["name"]]


def field_attnum_sort_key(field, column_order):
    for column in field_physical_columns(field):
        if column in column_order:
            return column_order[column]
    return NO_COLUMN_SORT_KEY


def load(name):
    with open(os.path.join(BASELINE, f"{name}-2.61.0.json")) as handle:
        return json.load(handle)


def rust_option(value, template="{}"):
    if value is None or value == "":
        return "None"
    escaped = str(value).replace("\\", "\\\\").replace('"', '\\"')
    return f'Some("{template.format(escaped)}")'


def rust_json(value):
    if value is None:
        return "None"
    text = json.dumps(value, separators=(",", ":"), sort_keys=True)
    escaped = text.replace("\\", "\\\\").replace('"', '\\"')
    return f'Some("{escaped}")'


def rust_bool(value):
    return "true" if value else "false"


def main():
    objects = load("object-metadata")
    fields = load("field-metadata")
    indexes = load("index-metadata")
    options = load("field-metadata-option")
    index_fields = load("index-field-metadata")
    search_fields = load("search-field-metadata")

    column_orders = parse_baseline_column_orders(
        os.path.join(BASELINE, "workspace-schema-2.61.0.sql")
    )

    field_universal_by_id = {f["id"]: f["universalIdentifier"] for f in fields}
    object_universal_by_id = {o["id"]: o["universalIdentifier"] for o in objects}

    fields_by_object = {}
    for field in fields:
        fields_by_object.setdefault(field["objectMetadataId"], []).append(field)

    index_fields_by_index = {}
    for entry in index_fields:
        index_fields_by_index.setdefault(entry["indexMetadataId"], []).append(entry)

    lines = []
    lines.append("pub struct StandardFieldDefinition {")
    lines.append("    pub universal_identifier: &'static str,")
    lines.append("    pub name: &'static str,")
    lines.append("    pub label: &'static str,")
    lines.append("    pub field_type: &'static str,")
    lines.append("    pub description: Option<&'static str>,")
    lines.append("    pub icon: Option<&'static str>,")
    lines.append("    pub is_active: bool,")
    lines.append("    pub is_system: bool,")
    lines.append("    pub is_nullable: bool,")
    lines.append("    pub is_audit_logged: bool,")
    lines.append("    pub is_ui_editable: bool,")
    lines.append("    pub is_label_synced_with_name: bool,")
    lines.append("    pub is_system_side_effect: bool,")
    lines.append("    pub default_value_json: Option<&'static str>,")
    lines.append("    pub options_json: Option<&'static str>,")
    lines.append("    pub settings_json: Option<&'static str>,")
    lines.append("    pub overrides_json: Option<&'static str>,")
    lines.append("    pub morph_id: Option<&'static str>,")
    lines.append(
        "    pub relation_target_field_universal_identifier: Option<&'static str>,"
    )
    lines.append(
        "    pub relation_target_object_universal_identifier: Option<&'static str>,"
    )
    lines.append("}")
    lines.append("")
    lines.append("pub struct StandardObjectDefinition {")
    lines.append("    pub universal_identifier: &'static str,")
    lines.append("    pub name_singular: &'static str,")
    lines.append("    pub name_plural: &'static str,")
    lines.append("    pub label_singular: &'static str,")
    lines.append("    pub label_plural: &'static str,")
    lines.append("    pub description: Option<&'static str>,")
    lines.append("    pub icon: Option<&'static str>,")
    lines.append("    pub color: Option<&'static str>,")
    lines.append("    pub target_table_name: &'static str,")
    lines.append("    pub is_system: bool,")
    lines.append("    pub is_active: bool,")
    lines.append("    pub is_searchable: bool,")
    lines.append("    pub is_audit_logged: bool,")
    lines.append("    pub is_label_synced_with_name: bool,")
    lines.append("    pub open_record_in: Option<&'static str>,")
    lines.append("    pub duplicate_criteria_json: Option<&'static str>,")
    lines.append("    pub overrides_json: Option<&'static str>,")
    lines.append("    pub shortcut: Option<&'static str>,")
    lines.append(
        "    pub label_identifier_field_universal_identifier: Option<&'static str>,"
    )
    lines.append(
        "    pub image_identifier_field_universal_identifier: Option<&'static str>,"
    )
    lines.append("    pub fields: &'static [StandardFieldDefinition],")
    lines.append("}")
    lines.append("")
    lines.append("pub struct StandardFieldOptionDefinition {")
    lines.append("    pub universal_identifier: &'static str,")
    lines.append("    pub field_universal_identifier: &'static str,")
    lines.append("    pub value: &'static str,")
    lines.append("    pub label: &'static str,")
    lines.append("    pub color: Option<&'static str>,")
    lines.append("    pub position: f64,")
    lines.append("}")
    lines.append("")
    lines.append("pub struct StandardIndexDefinition {")
    lines.append("    pub universal_identifier: &'static str,")
    lines.append("    pub name: &'static str,")
    lines.append("    pub object_universal_identifier: &'static str,")
    lines.append("    pub is_unique: bool,")
    lines.append("    pub index_where_clause: Option<&'static str>,")
    lines.append("    pub index_type: &'static str,")
    lines.append("    pub columns: &'static [StandardIndexColumnDefinition],")
    lines.append("}")
    lines.append("")
    lines.append("pub struct StandardIndexColumnDefinition {")
    lines.append("    pub order: i32,")
    lines.append("    pub field_universal_identifier: &'static str,")
    lines.append("    pub sub_field_name: Option<&'static str>,")
    lines.append("}")
    lines.append("")
    lines.append("pub struct StandardSearchFieldDefinition {")
    lines.append("    pub universal_identifier: &'static str,")
    lines.append("    pub object_universal_identifier: &'static str,")
    lines.append("    pub field_universal_identifier: &'static str,")
    lines.append("    pub ts_vector_field_universal_identifier: &'static str,")
    lines.append("    pub position: f64,")
    lines.append("}")
    lines.append("")
    lines.append(
        f'pub const STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER: &str = "{STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER}";'
    )
    lines.append("")

    for object_row in sorted(objects, key=lambda row: row["nameSingular"]):
        suffix = "".join(part.capitalize() for part in _split_name(object_row["nameSingular"])).upper()
        column_order = column_orders.get(object_row["nameSingular"], {})
        lines.append(
            f"pub const OBJECT_FIELDS_{suffix}: &[StandardFieldDefinition] = &["
        )
        for field in sorted(
            fields_by_object.get(object_row["id"], []),
            key=lambda row: (
                field_attnum_sort_key(row, column_order),
                row["name"],
            ),
        ):
            lines.append("    StandardFieldDefinition {")
            lines.append(
                f'        universal_identifier: "{field["universalIdentifier"]}",'
            )
            lines.append(f'        name: "{field["name"]}",')
            lines.append(f'        label: "{_escape(field["label"])}",')
            lines.append(f'        field_type: "{field["type"]}",')
            lines.append(
                f"        description: {rust_option(field.get('description'))},"
            )
            lines.append(f"        icon: {rust_option(field.get('icon'))},")
            lines.append(f"        is_active: {rust_bool(field['isActive'])},")
            lines.append(f"        is_system: {rust_bool(field['isSystem'])},")
            lines.append(
                f"        is_nullable: {rust_bool(field['isNullable'] if field['isNullable'] is not None else True)},"
            )
            lines.append(
                f"        is_audit_logged: {rust_bool(field['isAuditLogged'])},")
            lines.append(
                f"        is_ui_editable: {rust_bool(field['isUIEditable'])},")
            lines.append(
                f"        is_label_synced_with_name: {rust_bool(field['isLabelSyncedWithName'])},")
            lines.append(
                f"        is_system_side_effect: {rust_bool(field['isSystemSideEffect'])},")
            lines.append(f"        default_value_json: {rust_json(field.get('defaultValue'))},")
            lines.append(f"        options_json: {rust_json(field.get('options'))},")
            lines.append(f"        settings_json: {rust_json(field.get('settings'))},")
            lines.append(f"        overrides_json: {rust_json(field.get('overrides'))},")
            lines.append(
                f"        morph_id: {rust_option(field.get('morphId'))},"
            )

            target_field_id = field.get("relationTargetFieldMetadataId")
            target_field = (
                field_universal_by_id.get(target_field_id)
                if target_field_id
                else None
            )
            lines.append(
                f"        relation_target_field_universal_identifier: {rust_option(target_field)},"
            )

            target_object_id = field.get("relationTargetObjectMetadataId")
            target_object = (
                object_universal_by_id.get(target_object_id)
                if target_object_id
                else None
            )
            lines.append(
                f"        relation_target_object_universal_identifier: {rust_option(target_object)},"
            )
            lines.append("    },")
        lines.append("];")
        lines.append("")

    lines.append("pub const STANDARD_OBJECTS: &[StandardObjectDefinition] = &[")
    for object_row in sorted(objects, key=lambda row: row["nameSingular"]):
        suffix = "".join(part.capitalize() for part in _split_name(object_row["nameSingular"])).upper()
        label_field = field_universal_by_id.get(
            object_row.get("labelIdentifierFieldMetadataId")
        ) if object_row.get("labelIdentifierFieldMetadataId") else None
        image_field = field_universal_by_id.get(
            object_row.get("imageIdentifierFieldMetadataId")
        ) if object_row.get("imageIdentifierFieldMetadataId") else None
        lines.append("    StandardObjectDefinition {")
        lines.append(
            f'        universal_identifier: "{object_row["universalIdentifier"]}",'
        )
        lines.append(f'        name_singular: "{object_row["nameSingular"]}",')
        lines.append(f'        name_plural: "{object_row["namePlural"]}",')
        lines.append(f'        label_singular: "{_escape(object_row["labelSingular"])}",')
        lines.append(f'        label_plural: "{_escape(object_row["labelPlural"])}",')
        lines.append(
            f"        description: {rust_option(object_row.get('description'))},"
        )
        lines.append(f"        icon: {rust_option(object_row.get('icon'))},")
        lines.append(f"        color: {rust_option(object_row.get('color'))},")
        lines.append(
            f'        target_table_name: "{object_row["targetTableName"]}",'
        )
        lines.append(f"        is_system: {rust_bool(object_row['isSystem'])},")
        lines.append(f"        is_active: {rust_bool(object_row['isActive'])},")
        lines.append(
            f"        is_searchable: {rust_bool(object_row['isSearchable'])},")
        lines.append(
            f"        is_audit_logged: {rust_bool(object_row['isAuditLogged'])},")
        lines.append(
            f"        is_label_synced_with_name: {rust_bool(object_row['isLabelSyncedWithName'])},")
        lines.append(
            f"        open_record_in: {rust_option(object_row.get('openRecordIn'))},"
        )
        lines.append(
            f"        duplicate_criteria_json: {rust_json(object_row.get('duplicateCriteria'))},"
        )
        lines.append(
            f"        overrides_json: {rust_json(object_row.get('overrides'))},")
        lines.append(f"        shortcut: {rust_option(object_row.get('shortcut'))},")
        lines.append(
            f"        label_identifier_field_universal_identifier: {rust_option(label_field)},"
        )
        lines.append(
            f"        image_identifier_field_universal_identifier: {rust_option(image_field)},"
        )
        lines.append(f"        fields: OBJECT_FIELDS_{suffix},")
        lines.append("    },")
    lines.append("];")
    lines.append("")

    lines.append("pub const STANDARD_FIELD_OPTIONS: &[StandardFieldOptionDefinition] = &[")
    for option in sorted(options, key=lambda row: (row["fieldMetadataId"], row["position"])):
        field_uid = field_universal_by_id.get(option["fieldMetadataId"])
        if not field_uid:
            continue
        lines.append("    StandardFieldOptionDefinition {")
        lines.append(f'        universal_identifier: "{option["universalIdentifier"]}",')
        lines.append(f'        field_universal_identifier: "{field_uid}",')
        lines.append(f'        value: "{_escape(str(option["value"]))}",')
        lines.append(f'        label: "{_escape(option["label"])}",')
        lines.append(f"        color: {rust_option(option.get('color'))},")
        lines.append(f"        position: {float(option['position'])},")
        lines.append("    },")
    lines.append("];")
    lines.append("")

    for index_row in sorted(indexes, key=lambda row: row["name"]):
        columns = sorted(
            index_fields_by_index.get(index_row["id"], []),
            key=lambda row: row.get("order", 0),
        )
        column_constant = "COLUMNS_" + "".join(
            part.capitalize() for part in _split_name(index_row["name"])
        ).upper()
        lines.append(f"const {column_constant}: &[StandardIndexColumnDefinition] = &[")
        for column in columns:
            field_uid = field_universal_by_id.get(column["fieldMetadataId"])
            if not field_uid:
                continue
            lines.append("    StandardIndexColumnDefinition {")
            lines.append(f"        order: {int(column.get('order', 0))},")
            lines.append(f'        field_universal_identifier: "{field_uid}",')
            lines.append(
                f"        sub_field_name: {rust_option(column.get('subFieldName'))},"
            )
            lines.append("    },")
        lines.append("];")
        lines.append("")

    lines.append("pub const STANDARD_INDEXES: &[StandardIndexDefinition] = &[")

    for index_row in sorted(indexes, key=lambda row: row["name"]):
        object_uid = object_universal_by_id.get(index_row["objectMetadataId"])
        if not object_uid:
            continue
        column_constant = "COLUMNS_" + "".join(
            part.capitalize() for part in _split_name(index_row["name"])
        ).upper()
        lines.append("    StandardIndexDefinition {")
        lines.append(f'        universal_identifier: "{index_row["universalIdentifier"]}",')
        lines.append(f'        name: "{index_row["name"]}",')
        lines.append(f'        object_universal_identifier: "{object_uid}",')
        lines.append(f"        is_unique: {rust_bool(index_row['isUnique'])},")
        lines.append(
            f"        index_where_clause: {rust_option(index_row.get('indexWhereClause'))},"
        )
        lines.append(f'        index_type: "{index_row["indexType"]}",')
        lines.append(f"        columns: {column_constant},")
        lines.append("    },")
    lines.append("];")
    lines.append("")

    lines.append(
        "pub const STANDARD_SEARCH_FIELDS: &[StandardSearchFieldDefinition] = &["
    )
    for search_field in sorted(
        search_fields, key=lambda row: (row["objectMetadataId"], row["position"])
    ):
        object_uid = object_universal_by_id.get(search_field["objectMetadataId"])
        field_uid = field_universal_by_id.get(search_field["fieldMetadataId"])
        vector_uid = field_universal_by_id.get(search_field["tsVectorFieldMetadataId"])
        if not (object_uid and field_uid and vector_uid):
            continue
        lines.append("    StandardSearchFieldDefinition {")
        lines.append(
            f'        universal_identifier: "{search_field["universalIdentifier"]}",'
        )
        lines.append(f'        object_universal_identifier: "{object_uid}",')
        lines.append(f'        field_universal_identifier: "{field_uid}",')
        lines.append(
            f'        ts_vector_field_universal_identifier: "{vector_uid}",'
        )
        lines.append(f"        position: {float(search_field['position'])},")
        lines.append("    },")
    lines.append("];")
    lines.append("")

    with open(OUTPUT, "w") as handle:
        handle.write("\n".join(lines))

    print(f"wrote {OUTPUT}: {len(lines)} lines")
    print(f"objects={len(objects)} fields={len(fields)} indexes={len(indexes)}")


def _escape(value):
    return str(value).replace("\\", "\\\\").replace('"', '\\"')


def _split_name(name):
    parts = []
    current = ""
    for char in name:
        if char.isupper() and current:
            parts.append(current)
            current = char
        else:
            current += char
    if current:
        parts.append(current)
    return parts


if __name__ == "__main__":
    main()
