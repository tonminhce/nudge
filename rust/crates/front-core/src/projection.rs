use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectMetadata {
    pub name_singular: String,
    pub name_plural: String,
    #[serde(default)]
    pub label_singular: String,
    #[serde(default)]
    pub label_plural: String,
    #[serde(default)]
    pub duplicate_criteria: Option<Value>,
    #[serde(default, alias = "fields")]
    pub fields_list: Vec<FieldMetadata>,
}

impl ObjectMetadata {
    pub fn from_metadata_response(data: &Value) -> Vec<Self> {
        data.get("objects")
            .or_else(|| data.get("data").and_then(|data| data.get("objects")))
            .and_then(|objects| objects.get("edges"))
            .and_then(Value::as_array)
            .map(|edges| {
                edges
                    .iter()
                    .filter_map(|edge| edge.get("node"))
                    .filter_map(|node| serde_json::from_value(node.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMetadata {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub settings: Option<Value>,
    #[serde(default)]
    pub relation: Option<RelationMetadata>,
    #[serde(default)]
    pub morph_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationMetadata {
    #[serde(rename = "type")]
    pub relation_type: String,
    pub target_object_metadata: Box<ObjectMetadata>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectionOptions {
    pub depth: usize,
    pub include_system_fields: bool,
}

impl Default for ProjectionOptions {
    fn default() -> Self {
        Self {
            depth: 0,
            include_system_fields: true,
        }
    }
}

pub fn project_node(object: &ObjectMetadata, options: ProjectionOptions) -> String {
    object
        .fields_list
        .iter()
        .filter(|field| options.include_system_fields || !is_system_field(&field.name))
        .filter(|field| field.morph_id.is_none())
        .filter_map(|field| project_field(field, options))
        .collect::<Vec<_>>()
        .join(" ")
}

fn project_field(field: &FieldMetadata, options: ProjectionOptions) -> Option<String> {
    if let Some(properties) = composite_properties(&field.field_type) {
        return Some(format!("{} {{ {} }}", field.name, properties.join(" ")));
    }

    match field
        .relation
        .as_ref()
        .map(|relation| relation.relation_type.as_str())
    {
        Some("MANY_TO_ONE") if options.depth > 0 => {
            let target = project_node(
                &field.relation.as_ref().unwrap().target_object_metadata,
                ProjectionOptions {
                    depth: options.depth - 1,
                    include_system_fields: options.include_system_fields,
                },
            );
            Some(format!("{} {{ {target} }}", field.name))
        }
        Some("MANY_TO_ONE") => Some(format!("{}Id", field.name)),
        Some("ONE_TO_MANY") => None,
        _ => Some(field.name.clone()),
    }
}

fn composite_properties(field_type: &str) -> Option<&'static [&'static str]> {
    match field_type {
        "FULL_NAME" => Some(&["firstName", "lastName"]),
        "LINKS" => Some(&["primaryLinkLabel", "primaryLinkUrl", "secondaryLinks"]),
        "EMAILS" => Some(&["primaryEmail", "additionalEmails"]),
        "PHONES" => Some(&[
            "primaryPhoneNumber",
            "primaryPhoneCountryCode",
            "primaryPhoneCallingCode",
            "additionalPhones",
        ]),
        "ADDRESS" => Some(&[
            "addressStreet1",
            "addressStreet2",
            "addressCity",
            "addressState",
            "addressCountry",
            "addressPostcode",
            "addressLat",
            "addressLng",
        ]),
        "CURRENCY" => Some(&["amountMicros", "currencyCode"]),
        "ACTOR" => Some(&["source", "workspaceMemberId", "name", "context"]),
        "RICH_TEXT" => Some(&["blocknote", "markdown", "tiptap"]),
        _ => None,
    }
}

fn is_system_field(name: &str) -> bool {
    matches!(
        name,
        "id" | "createdAt" | "updatedAt" | "deletedAt" | "createdBy" | "updatedBy" | "position"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn person() -> ObjectMetadata {
        serde_json::from_value(json!({
            "nameSingular": "person",
            "namePlural": "people",
            "fieldsList": [
                {"name": "id", "type": "UUID"},
                {"name": "createdAt", "type": "DATE_TIME"},
                {"name": "name", "type": "FULL_NAME"},
                {"name": "emails", "type": "EMAILS"},
                {"name": "phones", "type": "PHONES"},
                {"name": "linkedinLink", "type": "LINKS"},
                {"name": "jobTitle", "type": "TEXT"},
                {"name": "avatarFile", "type": "FILES"},
                {"name": "position", "type": "POSITION"},
                {"name": "createdBy", "type": "ACTOR"},
                {"name": "attachments", "type": "RELATION", "relation": {
                    "type": "ONE_TO_MANY",
                    "targetObjectMetadata": {"nameSingular": "attachment", "namePlural": "attachments", "fieldsList": [{"name": "id", "type": "UUID"}]}
                }},
                {"name": "company", "type": "RELATION", "relation": {
                    "type": "MANY_TO_ONE",
                    "targetObjectMetadata": {"nameSingular": "company", "namePlural": "companies", "fieldsList": [{"name": "id", "type": "UUID"}, {"name": "name", "type": "TEXT"}]}
                }}
            ]
        }))
        .unwrap()
    }

    #[test]
    fn projection_matches_the_reference_composite_field_shapes() {
        let projection = project_node(&person(), ProjectionOptions::default());
        assert!(projection.contains("name { firstName lastName }"));
        assert!(projection.contains("emails { primaryEmail additionalEmails }"));
        assert!(projection.contains("phones { primaryPhoneNumber primaryPhoneCountryCode primaryPhoneCallingCode additionalPhones }"));
        assert!(
            projection.contains("linkedinLink { primaryLinkLabel primaryLinkUrl secondaryLinks }")
        );
        assert!(projection.contains("createdBy { source workspaceMemberId name context }"));
    }

    #[test]
    fn projection_at_depth_zero_maps_many_to_one_to_the_upstream_join_column() {
        let projection = project_node(&person(), ProjectionOptions::default());
        assert!(projection.contains("companyId"));
        assert!(!projection.contains("company {"));
        assert!(!projection.contains("attachments"));
    }

    #[test]
    fn projection_at_depth_one_expands_the_company_relation() {
        let projection = project_node(
            &person(),
            ProjectionOptions {
                depth: 1,
                include_system_fields: true,
            },
        );
        assert!(projection.contains("company { id name }"));
    }

    #[test]
    fn projection_can_omit_the_upstream_system_columns() {
        let projection = project_node(
            &person(),
            ProjectionOptions {
                depth: 0,
                include_system_fields: false,
            },
        );
        assert!(!projection.contains("createdAt"));
        assert!(!projection.contains("createdBy"));
        assert!(projection.contains("jobTitle"));
    }
}
