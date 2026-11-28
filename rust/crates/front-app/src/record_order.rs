use front_core::projection::FieldMetadata;
use front_core::projection::ObjectMetadata;
use serde_json::Map;
use serde_json::Value;

pub const ASC_NULLS_FIRST: OrderBy = OrderBy::AscNullsFirst;
pub const ASC_NULLS_LAST: OrderBy = OrderBy::AscNullsLast;
pub const DESC_NULLS_LAST: OrderBy = OrderBy::DescNullsLast;

const DEFAULT_LABEL_IDENTIFIER_FIELD_NAME: &str = "name";
const DEFAULT_PRIMARY_FULL_NAME_SUB_FIELD: &str = "firstName";
const DEFAULT_ADDRESS_SUB_FIELD: &str = "addressCity";
const POSITION_ORDER_BY_FIELD: &str = "position";
const FULL_NAME_SUB_FIELDS: [&str; 2] = ["firstName", "lastName"];
const ALLOWED_ADDRESS_SUB_FIELDS: [&str; 8] = [
    "addressStreet1",
    "addressStreet2",
    "addressCity",
    "addressState",
    "addressPostcode",
    "addressCountry",
    "addressLat",
    "addressLng",
];
const DEFAULT_VISIBLE_ADDRESS_SUB_FIELDS: [&str; 6] = [
    "addressStreet1",
    "addressStreet2",
    "addressCity",
    "addressState",
    "addressPostcode",
    "addressCountry",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderBy {
    AscNullsFirst,
    AscNullsLast,
    DescNullsLast,
}

impl OrderBy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AscNullsFirst => "AscNullsFirst",
            Self::AscNullsLast => "AscNullsLast",
            Self::DescNullsLast => "DescNullsLast",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewSortDirection {
    Asc,
    Desc,
}

impl ViewSortDirection {
    pub fn order_by(self) -> OrderBy {
        match self {
            Self::Asc => OrderBy::AscNullsLast,
            Self::Desc => OrderBy::DescNullsLast,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordSortField {
    pub field_name: String,
    pub direction: ViewSortDirection,
    pub sub_field_name: Option<String>,
}

impl RecordSortField {
    pub fn new(field_name: &str, direction: ViewSortDirection) -> Self {
        Self {
            field_name: field_name.to_string(),
            direction,
            sub_field_name: None,
        }
    }
}

fn order_entry(key: &str, value: Value) -> Value {
    let mut entry = Map::new();
    entry.insert(key.to_string(), value);
    Value::Object(entry)
}

fn direction_entry(key: &str, direction: OrderBy) -> Value {
    order_entry(key, Value::String(direction.as_str().to_string()))
}

fn sub_field_entry(field_name: &str, sub_field: &str, direction: OrderBy) -> Value {
    order_entry(field_name, direction_entry(sub_field, direction))
}

pub fn resolve_primary_full_name_sort_sub_field(requested: Option<&str>) -> &'static str {
    FULL_NAME_SUB_FIELDS
        .iter()
        .copied()
        .find(|sub_field| Some(*sub_field) == requested)
        .unwrap_or(DEFAULT_PRIMARY_FULL_NAME_SUB_FIELD)
}

pub fn resolve_secondary_full_name_sort_sub_field(primary: &str) -> &'static str {
    if primary == "firstName" {
        "lastName"
    } else {
        "firstName"
    }
}

pub fn get_enabled_address_sub_fields(settings: Option<&Value>) -> Vec<&str> {
    let enabled = settings
        .and_then(|settings| settings.get("subFields"))
        .and_then(Value::as_array)
        .filter(|sub_fields| !sub_fields.is_empty())
        .map(|sub_fields| {
            sub_fields
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if enabled.is_empty() {
        return DEFAULT_VISIBLE_ADDRESS_SUB_FIELDS.to_vec();
    }
    enabled
}

pub fn resolve_address_sort_sub_field<'a>(
    settings: Option<&'a Value>,
    requested: Option<&'a str>,
) -> &'a str {
    let enabled_sub_fields = get_enabled_address_sub_fields(settings);
    let requested = requested.filter(|sub_field| ALLOWED_ADDRESS_SUB_FIELDS.contains(sub_field));
    if let Some(requested) = requested {
        if enabled_sub_fields.contains(&requested) {
            return requested;
        }
    }
    if enabled_sub_fields.contains(&DEFAULT_ADDRESS_SUB_FIELD) {
        return DEFAULT_ADDRESS_SUB_FIELD;
    }
    enabled_sub_fields
        .first()
        .copied()
        .unwrap_or(DEFAULT_ADDRESS_SUB_FIELD)
}

pub fn get_order_by_for_field_metadata_type(
    field: &FieldMetadata,
    direction: Option<OrderBy>,
    primary_composite_sub_field: Option<&str>,
) -> Vec<Value> {
    let direction = direction.unwrap_or(ASC_NULLS_LAST);
    match field.field_type.as_str() {
        "FULL_NAME" => {
            let primary = resolve_primary_full_name_sort_sub_field(primary_composite_sub_field);
            let secondary = resolve_secondary_full_name_sort_sub_field(primary);
            vec![
                sub_field_entry(&field.name, primary, direction),
                sub_field_entry(&field.name, secondary, direction),
            ]
        }
        "ADDRESS" => vec![sub_field_entry(
            &field.name,
            resolve_address_sort_sub_field(field.settings.as_ref(), primary_composite_sub_field),
            direction,
        )],
        "CURRENCY" => vec![sub_field_entry(&field.name, "amountMicros", direction)],
        "ACTOR" => vec![sub_field_entry(&field.name, "name", direction)],
        "LINKS" => vec![sub_field_entry(&field.name, "primaryLinkUrl", direction)],
        "EMAILS" => vec![sub_field_entry(&field.name, "primaryEmail", direction)],
        "PHONES" => vec![sub_field_entry(
            &field.name,
            "primaryPhoneNumber",
            direction,
        )],
        _ => vec![direction_entry(&field.name, direction)],
    }
}

pub fn label_identifier_field(object: &ObjectMetadata) -> Option<&FieldMetadata> {
    object
        .fields_list
        .iter()
        .find(|field| field.name == DEFAULT_LABEL_IDENTIFIER_FIELD_NAME)
}

pub fn get_order_by_for_relation_field(
    field: &FieldMetadata,
    related_object: Option<&ObjectMetadata>,
    direction: OrderBy,
) -> Vec<Value> {
    let Some(label_field) = related_object.and_then(label_identifier_field) else {
        return vec![direction_entry(&format!("{}Id", field.name), direction)];
    };
    get_order_by_for_field_metadata_type(label_field, Some(direction), None)
        .into_iter()
        .map(|entry| order_entry(&field.name, entry))
        .collect()
}

pub fn has_object_metadata_item_position_field(object: &ObjectMetadata) -> bool {
    object
        .fields_list
        .iter()
        .any(|field| field.field_type == "POSITION")
}

pub fn turn_sorts_into_order_by(object: &ObjectMetadata, sorts: &[RecordSortField]) -> Vec<Value> {
    let mut entries = sorts
        .iter()
        .flat_map(|sort| {
            let Some(field) = object
                .fields_list
                .iter()
                .find(|field| field.name == sort.field_name)
            else {
                return Vec::new();
            };
            let direction = sort.direction.order_by();
            if field.field_type == "RELATION" {
                return get_order_by_for_relation_field(
                    field,
                    field
                        .relation
                        .as_ref()
                        .map(|relation| relation.target_object_metadata.as_ref()),
                    direction,
                );
            }
            get_order_by_for_field_metadata_type(
                field,
                Some(direction),
                sort.sub_field_name.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    append_position_tiebreaker(object, &mut entries);
    entries
}

fn append_position_tiebreaker(object: &ObjectMetadata, entries: &mut Vec<Value>) {
    if has_object_metadata_item_position_field(object) {
        entries.push(direction_entry(POSITION_ORDER_BY_FIELD, ASC_NULLS_FIRST));
    }
}

pub fn get_order_by_field_for_object(
    object: &ObjectMetadata,
    direction: Option<OrderBy>,
) -> Vec<Value> {
    match label_identifier_field(object) {
        Some(field) => get_order_by_for_field_metadata_type(field, direction, None),
        None => vec![direction_entry(
            "createdAt",
            direction.unwrap_or(DESC_NULLS_LAST),
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use front_core::projection::RelationMetadata;

    fn field(name: &str, field_type: &str) -> FieldMetadata {
        FieldMetadata {
            name: name.to_string(),
            field_type: field_type.to_string(),
            settings: None,
            relation: None,
            morph_id: None,
        }
    }

    fn field_with_settings(name: &str, field_type: &str, settings: Value) -> FieldMetadata {
        FieldMetadata {
            settings: Some(settings),
            ..field(name, field_type)
        }
    }

    fn object(name_singular: &str, fields: Vec<FieldMetadata>) -> ObjectMetadata {
        ObjectMetadata {
            name_singular: name_singular.to_string(),
            name_plural: format!("{name_singular}s"),
            duplicate_criteria: None,
            fields_list: fields,
        }
    }

    fn relation_field(name: &str, related: ObjectMetadata) -> FieldMetadata {
        FieldMetadata {
            relation: Some(RelationMetadata {
                relation_type: "MANY_TO_ONE".to_string(),
                target_object_metadata: Box::new(related),
            }),
            ..field(name, "RELATION")
        }
    }

    #[test]
    fn view_sort_directions_are_nulls_last_in_both_directions() {
        assert_eq!(ViewSortDirection::Asc.order_by().as_str(), "AscNullsLast");
        assert_eq!(ViewSortDirection::Desc.order_by().as_str(), "DescNullsLast");
    }

    #[test]
    fn full_name_sort_expands_to_primary_then_secondary_sub_fields() {
        let entries = get_order_by_for_field_metadata_type(
            &field("name", "FULL_NAME"),
            Some(ASC_NULLS_LAST),
            None,
        );
        assert_eq!(
            entries,
            vec![
                serde_json::json!({"name": {"firstName": "AscNullsLast"}}),
                serde_json::json!({"name": {"lastName": "AscNullsLast"}}),
            ]
        );
    }

    #[test]
    fn a_requested_full_name_sub_field_leads_the_pair() {
        let entries = get_order_by_for_field_metadata_type(
            &field("name", "FULL_NAME"),
            Some(DESC_NULLS_LAST),
            Some("lastName"),
        );
        assert_eq!(
            entries,
            vec![
                serde_json::json!({"name": {"lastName": "DescNullsLast"}}),
                serde_json::json!({"name": {"firstName": "DescNullsLast"}}),
            ]
        );
    }

    #[test]
    fn an_unsupported_full_name_sub_field_falls_back_to_first_name() {
        let entries = get_order_by_for_field_metadata_type(
            &field("name", "FULL_NAME"),
            Some(ASC_NULLS_LAST),
            Some("middleName"),
        );
        assert_eq!(
            entries,
            vec![
                serde_json::json!({"name": {"firstName": "AscNullsLast"}}),
                serde_json::json!({"name": {"lastName": "AscNullsLast"}}),
            ]
        );
    }

    #[test]
    fn currency_actor_links_emails_and_phones_use_their_composite_leaves() {
        let cases = [
            ("CURRENCY", "amountMicros"),
            ("ACTOR", "name"),
            ("LINKS", "primaryLinkUrl"),
            ("EMAILS", "primaryEmail"),
            ("PHONES", "primaryPhoneNumber"),
        ];
        for (field_type, sub_field) in cases {
            let entries = get_order_by_for_field_metadata_type(
                &field("column", field_type),
                Some(DESC_NULLS_LAST),
                None,
            );
            assert_eq!(
                entries,
                vec![serde_json::json!({
                    "column": { sub_field: "DescNullsLast" }
                })],
                "unexpected order by for {field_type}"
            );
        }
    }

    #[test]
    fn scalar_fields_order_by_the_field_name_itself() {
        for field_type in ["TEXT", "VARCHAR", "DATE_TIME", "NUMBER", "SELECT", "UUID"] {
            assert_eq!(
                get_order_by_for_field_metadata_type(
                    &field("anyField", field_type),
                    Some(DESC_NULLS_LAST),
                    None,
                ),
                vec![serde_json::json!({"anyField": "DescNullsLast"})],
                "unexpected order by for {field_type}"
            );
        }
    }

    #[test]
    fn a_missing_direction_falls_back_to_asc_nulls_last() {
        let scalar = get_order_by_for_field_metadata_type(&field("stage", "SELECT"), None, None);
        assert_eq!(scalar, vec![serde_json::json!({"stage": "AscNullsLast"})]);
        let composite =
            get_order_by_for_field_metadata_type(&field("name", "FULL_NAME"), None, None);
        assert_eq!(
            composite,
            vec![
                serde_json::json!({"name": {"firstName": "AscNullsLast"}}),
                serde_json::json!({"name": {"lastName": "AscNullsLast"}}),
            ]
        );
        let currency =
            get_order_by_for_field_metadata_type(&field("amount", "CURRENCY"), None, None);
        assert_eq!(
            currency,
            vec![serde_json::json!({"amount": {"amountMicros": "AscNullsLast"}})]
        );
    }

    #[test]
    fn address_defaults_to_the_city_sub_field() {
        let entries = get_order_by_for_field_metadata_type(
            &field("address", "ADDRESS"),
            Some(ASC_NULLS_LAST),
            None,
        );
        assert_eq!(
            entries,
            vec![serde_json::json!({"address": {"addressCity": "AscNullsLast"}})]
        );
    }

    #[test]
    fn address_follows_the_requested_sub_field_when_it_is_enabled() {
        let settings = serde_json::json!({"subFields": ["addressCity", "addressState"]});
        let entries = get_order_by_for_field_metadata_type(
            &field_with_settings("address", "ADDRESS", settings),
            Some(DESC_NULLS_LAST),
            Some("addressState"),
        );
        assert_eq!(
            entries,
            vec![serde_json::json!({"address": {"addressState": "DescNullsLast"}})]
        );
    }

    #[test]
    fn address_ignores_a_disabled_or_unknown_sub_field() {
        let settings = serde_json::json!({"subFields": ["addressCountry", "addressPostcode"]});
        let disabled = get_order_by_for_field_metadata_type(
            &field_with_settings(
                "address",
                "ADDRESS",
                serde_json::json!({"subFields": ["addressCountry"]}),
            ),
            Some(ASC_NULLS_LAST),
            Some("addressCity"),
        );
        assert_eq!(
            disabled,
            vec![serde_json::json!({"address": {"addressCountry": "AscNullsLast"}})]
        );
        let unknown = get_order_by_for_field_metadata_type(
            &field_with_settings("address", "ADDRESS", settings),
            Some(ASC_NULLS_LAST),
            Some("addressLatitude"),
        );
        assert_eq!(
            unknown,
            vec![serde_json::json!({"address": {"addressCountry": "AscNullsLast"}})]
        );
    }

    #[test]
    fn address_without_settings_uses_the_default_visible_sub_fields() {
        assert_eq!(
            get_enabled_address_sub_fields(None),
            vec![
                "addressStreet1",
                "addressStreet2",
                "addressCity",
                "addressState",
                "addressPostcode",
                "addressCountry",
            ]
        );
        assert_eq!(
            get_enabled_address_sub_fields(Some(&serde_json::json!({"subFields": []}))),
            DEFAULT_VISIBLE_ADDRESS_SUB_FIELDS.to_vec()
        );
    }

    #[test]
    fn relation_fields_order_through_the_related_label_identifier() {
        let company = object("company", vec![field("name", "TEXT")]);
        let entries = get_order_by_for_relation_field(
            &relation_field("company", company),
            Some(&object("company", vec![field("name", "TEXT")])),
            ASC_NULLS_LAST,
        );
        assert_eq!(
            entries,
            vec![serde_json::json!({"company": {"name": "AscNullsLast"}})]
        );
    }

    #[test]
    fn relation_to_a_composite_label_identifier_keeps_both_sub_fields() {
        let person = object("person", vec![field("name", "FULL_NAME")]);
        let entries = get_order_by_for_relation_field(
            &field("pointOfContact", "RELATION"),
            Some(&person),
            DESC_NULLS_LAST,
        );
        assert_eq!(
            entries,
            vec![
                serde_json::json!({"pointOfContact": {"name": {"firstName": "DescNullsLast"}}}),
                serde_json::json!({"pointOfContact": {"name": {"lastName": "DescNullsLast"}}}),
            ]
        );
    }

    #[test]
    fn a_relation_without_a_related_label_identifier_falls_back_to_the_fk() {
        let without_name = object("company", vec![field("domainName", "TEXT")]);
        assert_eq!(
            get_order_by_for_relation_field(
                &field("company", "RELATION"),
                Some(&without_name),
                ASC_NULLS_LAST
            ),
            vec![serde_json::json!({"companyId": "AscNullsLast"})]
        );
        assert_eq!(
            get_order_by_for_relation_field(&field("company", "RELATION"), None, DESC_NULLS_LAST),
            vec![serde_json::json!({"companyId": "DescNullsLast"})]
        );
    }

    #[test]
    fn sorts_resolve_their_order_by_from_the_field_metadata_type() {
        let person = object(
            "person",
            vec![
                field("name", "FULL_NAME"),
                field("emails", "EMAILS"),
                field("position", "POSITION"),
            ],
        );
        let sorts = vec![
            RecordSortField::new("name", ViewSortDirection::Desc),
            RecordSortField::new("emails", ViewSortDirection::Asc),
        ];
        assert_eq!(
            turn_sorts_into_order_by(&person, &sorts),
            vec![
                serde_json::json!({"name": {"firstName": "DescNullsLast"}}),
                serde_json::json!({"name": {"lastName": "DescNullsLast"}}),
                serde_json::json!({"emails": {"primaryEmail": "AscNullsLast"}}),
                serde_json::json!({"position": "AscNullsFirst"}),
            ]
        );
    }

    #[test]
    fn a_sort_on_an_unknown_field_is_dropped_but_the_position_tiebreaker_stays() {
        let person = object(
            "person",
            vec![field("name", "FULL_NAME"), field("position", "POSITION")],
        );
        assert_eq!(
            turn_sorts_into_order_by(
                &person,
                &[RecordSortField::new("ghost", ViewSortDirection::Asc)]
            ),
            vec![serde_json::json!({"position": "AscNullsFirst"})]
        );
    }

    #[test]
    fn the_position_tiebreaker_is_absent_without_a_position_field() {
        let workflow = object("workflowVersion", vec![field("name", "TEXT")]);
        assert_eq!(
            turn_sorts_into_order_by(
                &workflow,
                &[RecordSortField::new("name", ViewSortDirection::Asc)]
            ),
            vec![serde_json::json!({"name": "AscNullsLast"})]
        );
        assert!(!has_object_metadata_item_position_field(&workflow));
    }

    #[test]
    fn relation_sorts_reach_the_order_by_through_the_embedded_metadata() {
        let person = object(
            "person",
            vec![
                relation_field("company", object("company", vec![field("name", "TEXT")])),
                field("position", "POSITION"),
            ],
        );
        assert_eq!(
            turn_sorts_into_order_by(
                &person,
                &[RecordSortField::new("company", ViewSortDirection::Desc)]
            ),
            vec![
                serde_json::json!({"company": {"name": "DescNullsLast"}}),
                serde_json::json!({"position": "AscNullsFirst"}),
            ]
        );
    }

    #[test]
    fn an_address_sort_carries_the_requested_sub_field_from_the_dropdown() {
        let company = object(
            "company",
            vec![
                field_with_settings(
                    "address",
                    "ADDRESS",
                    serde_json::json!({"subFields": ["addressState", "addressCity"]}),
                ),
                field("position", "POSITION"),
            ],
        );
        let sorts = vec![RecordSortField {
            field_name: String::from("address"),
            direction: ViewSortDirection::Asc,
            sub_field_name: Some(String::from("addressState")),
        }];
        assert_eq!(
            turn_sorts_into_order_by(&company, &sorts),
            vec![
                serde_json::json!({"address": {"addressState": "AscNullsLast"}}),
                serde_json::json!({"position": "AscNullsFirst"}),
            ]
        );
    }

    #[test]
    fn the_default_order_by_is_the_label_identifier_or_created_at_desc() {
        let company = object(
            "company",
            vec![field("name", "TEXT"), field("position", "POSITION")],
        );
        assert_eq!(
            get_order_by_field_for_object(&company, Some(DESC_NULLS_LAST)),
            vec![serde_json::json!({"name": "DescNullsLast"})]
        );
        assert_eq!(
            get_order_by_field_for_object(&company, None),
            vec![serde_json::json!({"name": "AscNullsLast"})]
        );
        let note = object("note", vec![field("title", "TEXT")]);
        assert_eq!(
            get_order_by_field_for_object(&note, None),
            vec![serde_json::json!({"createdAt": "DescNullsLast"})]
        );
        assert_eq!(
            get_order_by_field_for_object(&note, Some(ASC_NULLS_LAST)),
            vec![serde_json::json!({"createdAt": "AscNullsLast"})]
        );
    }
}
