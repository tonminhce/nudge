#![allow(dead_code)]

pub mod composite_types;

use serde_json::Value;

use crate::metadata::names::{
    compute_column_name, compute_composite_column_name, compute_postgres_enum_name,
    compute_relation_join_column_name, compute_server_only_composite_column_name,
    remove_sql_ddl_injection,
};

use super::ddl::escape::{assert_safe_ts_vector_expression, escape_identifier, escape_literal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldMetadataType {
    Actor,
    Address,
    Array,
    Boolean,
    Currency,
    Date,
    DateTime,
    Emails,
    Files,
    FullName,
    Links,
    MorphRelation,
    MultiSelect,
    Number,
    Numeric,
    Phones,
    Position,
    Rating,
    RawJson,
    Relation,
    RichText,
    Select,
    Text,
    TsVector,
    Uuid,
}

impl FieldMetadataType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldMetadataType::Actor => "ACTOR",
            FieldMetadataType::Address => "ADDRESS",
            FieldMetadataType::Array => "ARRAY",
            FieldMetadataType::Boolean => "BOOLEAN",
            FieldMetadataType::Currency => "CURRENCY",
            FieldMetadataType::Date => "DATE",
            FieldMetadataType::DateTime => "DATE_TIME",
            FieldMetadataType::Emails => "EMAILS",
            FieldMetadataType::Files => "FILES",
            FieldMetadataType::FullName => "FULL_NAME",
            FieldMetadataType::Links => "LINKS",
            FieldMetadataType::MorphRelation => "MORPH_RELATION",
            FieldMetadataType::MultiSelect => "MULTI_SELECT",
            FieldMetadataType::Number => "NUMBER",
            FieldMetadataType::Numeric => "NUMERIC",
            FieldMetadataType::Phones => "PHONES",
            FieldMetadataType::Position => "POSITION",
            FieldMetadataType::Rating => "RATING",
            FieldMetadataType::RawJson => "RAW_JSON",
            FieldMetadataType::Relation => "RELATION",
            FieldMetadataType::RichText => "RICH_TEXT",
            FieldMetadataType::Select => "SELECT",
            FieldMetadataType::Text => "TEXT",
            FieldMetadataType::TsVector => "TS_VECTOR",
            FieldMetadataType::Uuid => "UUID",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedType {
    Stored,
    Virtual,
}

impl GeneratedType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GeneratedType::Stored => "STORED",
            GeneratedType::Virtual => "VIRTUAL",
        }
    }

    fn from_raw(raw: &str) -> Option<Self> {
        match raw {
            "STORED" => Some(GeneratedType::Stored),
            "VIRTUAL" => Some(GeneratedType::Virtual),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSchemaColumnDefinition {
    pub name: String,
    pub column_type: String,
    pub is_nullable: Option<bool>,
    pub default: Option<String>,
    pub is_primary: bool,
    pub is_array: bool,
    pub as_expression: Option<String>,
    pub generated_type: Option<GeneratedType>,
    pub is_generated_always_as_identity: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlatFieldColumnInput {
    pub name: String,
    pub field_type: FieldMetadataType,
    pub is_nullable: Option<bool>,
    pub default_value: Option<Value>,
    pub universal_identifier: String,
    pub morph_id: Option<String>,
    pub relation_type: Option<String>,
}

pub const MANY_TO_ONE_RELATION_TYPE: &str = "MANY_TO_ONE";

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnDefsError {
    UnsupportedFieldMetadataType { field_type: String },
    UnsupportedCompositeColumnType { field_type: String },
    InvalidDefaultValue { message: String },
    UnsafeTsVectorExpression,
    IdentifierEscape { message: String },
    LiteralEscape { message: String },
}

impl ColumnDefsError {
    pub fn code(&self) -> &'static str {
        match self {
            ColumnDefsError::UnsupportedFieldMetadataType { .. } => {
                "UNSUPPORTED_FIELD_METADATA_TYPE"
            }
            ColumnDefsError::UnsupportedCompositeColumnType { .. } => {
                "UNSUPPORTED_COMPOSITE_COLUMN_TYPE"
            }
            ColumnDefsError::InvalidDefaultValue { .. } => "INVALID_FIELD_INPUT",
            ColumnDefsError::UnsafeTsVectorExpression => "UNSAFE_TS_VECTOR_EXPRESSION",
            ColumnDefsError::IdentifierEscape { .. } => "INTERNAL_SERVER_ERROR",
            ColumnDefsError::LiteralEscape { .. } => "INTERNAL_SERVER_ERROR",
        }
    }
}

impl std::fmt::Display for ColumnDefsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnDefsError::UnsupportedFieldMetadataType { field_type } => {
                write!(formatter, "Cannot convert {field_type} to column type.")
            }
            ColumnDefsError::UnsupportedCompositeColumnType { field_type } => {
                write!(
                    formatter,
                    "Relation type not supported for composite columns: {field_type}"
                )
            }
            ColumnDefsError::InvalidDefaultValue { message } => write!(formatter, "{message}"),
            ColumnDefsError::UnsafeTsVectorExpression => {
                write!(formatter, "Unsafe tsvector expression detected")
            }
            ColumnDefsError::IdentifierEscape { message } => write!(formatter, "{message}"),
            ColumnDefsError::LiteralEscape { message } => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for ColumnDefsError {}

impl From<crate::workspace_migrations::ddl::error::DdlError> for ColumnDefsError {
    fn from(error: crate::workspace_migrations::ddl::error::DdlError) -> Self {
        if error.code() == "UNSAFE_TS_VECTOR_EXPRESSION" {
            return ColumnDefsError::UnsafeTsVectorExpression;
        }

        ColumnDefsError::IdentifierEscape {
            message: error.to_string(),
        }
    }
}

pub fn is_text_column_type(field_type: FieldMetadataType) -> bool {
    matches!(
        field_type,
        FieldMetadataType::Text | FieldMetadataType::Array | FieldMetadataType::RichText
    )
}

pub fn is_text_column_type_raw(raw: &str) -> bool {
    raw == "TEXT" || raw == "ARRAY" || raw == "RICH_TEXT_V2" || raw == "RICH_TEXT"
}

pub fn field_metadata_type_to_column_type(
    field_type: FieldMetadataType,
) -> Result<&'static str, ColumnDefsError> {
    if is_text_column_type(field_type) {
        return Ok("text");
    }

    match field_type {
        FieldMetadataType::Uuid => Ok("uuid"),
        FieldMetadataType::Numeric => Ok("numeric"),
        FieldMetadataType::Number | FieldMetadataType::Position => Ok("float"),
        FieldMetadataType::Boolean => Ok("boolean"),
        FieldMetadataType::DateTime => Ok("timestamptz"),
        FieldMetadataType::Date => Ok("date"),
        FieldMetadataType::Rating | FieldMetadataType::Select | FieldMetadataType::MultiSelect => {
            Ok("enum")
        }
        FieldMetadataType::Files | FieldMetadataType::RawJson => Ok("jsonb"),
        FieldMetadataType::TsVector => Ok("tsvector"),
        other => Err(ColumnDefsError::UnsupportedFieldMetadataType {
            field_type: other.as_str().to_string(),
        }),
    }
}

pub fn is_function_default_value(value: &str) -> bool {
    !value.starts_with('\'') && (value == "uuid" || value == "now")
}

fn serialize_function_default_value(function_name: &str) -> Option<&'static str> {
    match function_name {
        "uuid" => Some("public.uuid_generate_v4()"),
        "now" => Some("now()"),
        _ => None,
    }
}

fn strip_surrounding_quotes(value: &str) -> &str {
    if value.starts_with('\'') && value.ends_with('\'') {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn js_scalar_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn enum_cast_suffix(
    schema_name: &str,
    table_name: &str,
    column_name: &str,
) -> Result<String, ColumnDefsError> {
    let enum_name = compute_postgres_enum_name(
        &remove_sql_ddl_injection(table_name),
        &remove_sql_ddl_injection(column_name),
    );

    Ok(format!(
        "::{}.{}",
        escape_identifier(schema_name)?,
        escape_identifier(&enum_name)?
    ))
}

fn cast_suffix(
    column_type: &str,
    schema_name: &str,
    table_name: &str,
    column_name: &str,
) -> Result<String, ColumnDefsError> {
    if column_type == "enum" {
        enum_cast_suffix(schema_name, table_name, column_name)
    } else {
        Ok(format!("::{column_type}"))
    }
}

pub fn serialize_default_value(
    default_value: Option<&Value>,
    column_type: &str,
    schema_name: &str,
    table_name: &str,
    column_name: &str,
) -> Result<String, ColumnDefsError> {
    let default_value = match default_value {
        None | Some(Value::Null) => return Ok("NULL".to_string()),
        Some(value) => value,
    };

    if let Value::String(raw) = default_value {
        if is_function_default_value(raw) {
            return serialize_function_default_value(raw)
                .map(str::to_string)
                .ok_or_else(|| ColumnDefsError::InvalidDefaultValue {
                    message: "Invalid default value".to_string(),
                });
        }
    }

    let cast = cast_suffix(column_type, schema_name, table_name, column_name)?;

    match default_value {
        Value::String(raw) => {
            if !raw.starts_with('\'') || !raw.ends_with('\'') {
                return Err(ColumnDefsError::InvalidDefaultValue {
                    message: format!(
                        "Invalid string default value \"{raw}\" should be single quoted"
                    ),
                });
            }

            Ok(format!(
                "{}{cast}",
                escape_literal(strip_surrounding_quotes(raw))?
            ))
        }
        Value::Bool(value) => Ok(format!("{}{cast}", escape_literal(&value.to_string())?)),
        Value::Number(value) => Ok(format!("{}{cast}", escape_literal(&value.to_string())?)),
        Value::Array(values) => {
            let mut array_values = String::new();

            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    array_values.push(',');
                }

                let escaped =
                    escape_literal(strip_surrounding_quotes(&js_scalar_to_string(value)))?;
                array_values.push_str(&escaped);
            }

            Ok(format!("ARRAY[{array_values}]{cast}[]"))
        }
        Value::Object(_) => Ok(format!(
            "{}{cast}",
            escape_literal(&default_value.to_string())?
        )),
        Value::Null => Ok("NULL".to_string()),
    }
}

pub fn build_sql_column_definition(
    column: &WorkspaceSchemaColumnDefinition,
) -> Result<String, ColumnDefsError> {
    let mut parts = vec![escape_identifier(&column.name)?];

    parts.push(if column.is_array {
        format!("{}[]", column.column_type)
    } else {
        column.column_type.clone()
    });

    if column.as_expression.is_some() && column.column_type == "tsvector" {
        let expression = column.as_expression.as_deref().unwrap_or_default();
        assert_safe_ts_vector_expression(expression)?;
        parts.push(format!("GENERATED ALWAYS AS ({expression})"));

        if let Some(generated_type) = column.generated_type {
            parts.push(generated_type.as_str().to_string());
        }
    }

    if column.is_generated_always_as_identity {
        parts.push("GENERATED ALWAYS AS IDENTITY".to_string());
    }

    if column.is_primary {
        parts.push("PRIMARY KEY".to_string());
    }

    if column.is_nullable == Some(false) {
        parts.push("NOT NULL".to_string());
    }

    if column.default.is_some() && column.column_type != "tsvector" {
        parts.push(format!(
            "DEFAULT {}",
            column.default.as_deref().unwrap_or_default()
        ));
    }

    Ok(parts.join(" "))
}

pub fn generate_composite_column_definition(
    property: &composite_types::CompositeProperty,
    parent_field: &FlatFieldColumnInput,
    schema_name: &str,
    table_name: &str,
) -> Result<WorkspaceSchemaColumnDefinition, ColumnDefsError> {
    if property.field_type == FieldMetadataType::Relation
        || property.field_type == FieldMetadataType::MorphRelation
    {
        return Err(ColumnDefsError::UnsupportedCompositeColumnType {
            field_type: property.field_type.as_str().to_string(),
        });
    }

    let column_name = compute_composite_column_name(&parent_field.name, property.name);
    let normalized_default = composite_types::nullify_empty_composite_default_value(
        parent_field.default_value.as_ref(),
        parent_field.field_type,
    );
    let default_value = normalized_default
        .as_ref()
        .and_then(|map| map.get(property.name));
    let column_type = field_metadata_type_to_column_type(property.field_type)?;
    let serialized_default = serialize_default_value(
        default_value,
        column_type,
        schema_name,
        table_name,
        &column_name,
    )?;

    let is_array = property.field_type == FieldMetadataType::Array
        || property.field_type == FieldMetadataType::MultiSelect
        || property.is_array;

    let resolved_column_type = if column_type == "enum" {
        format!(
            "\"{}\".\"{}\"",
            schema_name,
            compute_postgres_enum_name(table_name, &column_name)
        )
    } else {
        column_type.to_string()
    };

    Ok(WorkspaceSchemaColumnDefinition {
        name: column_name,
        column_type: resolved_column_type,
        is_nullable: Some(true),
        default: Some(serialized_default),
        is_primary: false,
        is_array,
        as_expression: None,
        generated_type: None,
        is_generated_always_as_identity: false,
    })
}

pub fn build_server_only_composite_column_definitions(
    field_type: FieldMetadataType,
    field_name: &str,
) -> Result<Vec<WorkspaceSchemaColumnDefinition>, ColumnDefsError> {
    let server_only_columns: [(&str, FieldMetadataType); 1] =
        [("fileIds", FieldMetadataType::RawJson)];

    if field_type != FieldMetadataType::RichText {
        return Ok(Vec::new());
    }

    server_only_columns
        .iter()
        .map(|(column_name, column_field_type)| {
            Ok(WorkspaceSchemaColumnDefinition {
                name: compute_server_only_composite_column_name(field_name, column_name),
                column_type: field_metadata_type_to_column_type(*column_field_type)?.to_string(),
                is_nullable: Some(true),
                default: None,
                is_primary: false,
                is_array: false,
                as_expression: None,
                generated_type: None,
                is_generated_always_as_identity: false,
            })
        })
        .collect()
}

pub fn generate_ts_vector_column_definition(
    field: &FlatFieldColumnInput,
    search_vector_expression: Option<&str>,
) -> WorkspaceSchemaColumnDefinition {
    WorkspaceSchemaColumnDefinition {
        name: compute_column_name(&field.name),
        column_type: "tsvector".to_string(),
        is_nullable: Some(true),
        default: None,
        is_primary: false,
        is_array: false,
        as_expression: search_vector_expression.map(str::to_string),
        generated_type: Some(GeneratedType::Stored),
        is_generated_always_as_identity: false,
    }
}

pub fn generate_relation_column_definition(
    field: &FlatFieldColumnInput,
) -> Option<WorkspaceSchemaColumnDefinition> {
    if field.relation_type.as_deref()? != MANY_TO_ONE_RELATION_TYPE {
        return None;
    }

    Some(WorkspaceSchemaColumnDefinition {
        name: compute_relation_join_column_name(&field.name),
        column_type: field_metadata_type_to_column_type(FieldMetadataType::Uuid)
            .unwrap_or("uuid")
            .to_string(),
        is_nullable: Some(true),
        default: None,
        is_primary: false,
        is_array: false,
        as_expression: None,
        generated_type: None,
        is_generated_always_as_identity: false,
    })
}

pub fn generate_scalar_column_definition(
    field: &FlatFieldColumnInput,
    schema_name: &str,
    table_name: &str,
) -> Result<WorkspaceSchemaColumnDefinition, ColumnDefsError> {
    let column_name = compute_column_name(&field.name);
    let column_type = field_metadata_type_to_column_type(field.field_type)?;
    let serialized_default = serialize_default_value(
        field.default_value.as_ref(),
        column_type,
        schema_name,
        table_name,
        &column_name,
    )?;

    let resolved_column_type = if column_type == "enum" {
        format!(
            "\"{}\".\"{}\"",
            schema_name,
            compute_postgres_enum_name(table_name, &column_name)
        )
    } else {
        column_type.to_string()
    };

    Ok(WorkspaceSchemaColumnDefinition {
        name: column_name,
        column_type: resolved_column_type,
        is_nullable: field.is_nullable,
        is_array: field.field_type == FieldMetadataType::Array
            || field.field_type == FieldMetadataType::MultiSelect,
        default: Some(serialized_default),
        is_primary: field.name == "id",
        as_expression: None,
        generated_type: None,
        is_generated_always_as_identity: false,
    })
}

pub fn is_morph_relation_head(field: &FlatFieldColumnInput) -> bool {
    field.field_type == FieldMetadataType::MorphRelation
        && field.morph_id.as_deref() == Some(field.universal_identifier.as_str())
}

pub fn generate_column_definitions(
    field: &FlatFieldColumnInput,
    schema_name: &str,
    table_name: &str,
    search_vector_expression: Option<&str>,
) -> Result<Vec<WorkspaceSchemaColumnDefinition>, ColumnDefsError> {
    if is_morph_relation_head(field) {
        return Ok(Vec::new());
    }

    if composite_types::is_composite_field_metadata_type(field.field_type) {
        let mut definitions = composite_types::composite_properties(field.field_type)
            .iter()
            .map(|property| {
                generate_composite_column_definition(property, field, schema_name, table_name)
            })
            .collect::<Result<Vec<_>, _>>()?;

        definitions.extend(build_server_only_composite_column_definitions(
            field.field_type,
            &field.name,
        )?);

        return Ok(definitions);
    }

    if field.field_type == FieldMetadataType::TsVector {
        return Ok(vec![generate_ts_vector_column_definition(
            field,
            search_vector_expression,
        )]);
    }

    if field.field_type == FieldMetadataType::Relation
        || field.field_type == FieldMetadataType::MorphRelation
    {
        return Ok(generate_relation_column_definition(field)
            .map(|definition| vec![definition])
            .unwrap_or_default());
    }

    Ok(vec![generate_scalar_column_definition(
        field,
        schema_name,
        table_name,
    )?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use composite_types::COMPOSITE_TYPES;
    use serde_json::json;

    const SCHEMA: &str = "workspace_1wgvd1injqtife6y4rvfbu3h5";

    fn field(name: &str, field_type: FieldMetadataType) -> FlatFieldColumnInput {
        FlatFieldColumnInput {
            name: name.to_string(),
            field_type,
            is_nullable: None,
            default_value: None,
            universal_identifier: format!("uid-{name}"),
            morph_id: None,
            relation_type: None,
        }
    }

    #[test]
    fn column_type_mapping_follows_the_upstream_table() {
        let vectors = [
            (FieldMetadataType::Text, "text"),
            (FieldMetadataType::Array, "text"),
            (FieldMetadataType::RichText, "text"),
            (FieldMetadataType::Uuid, "uuid"),
            (FieldMetadataType::Numeric, "numeric"),
            (FieldMetadataType::Number, "float"),
            (FieldMetadataType::Position, "float"),
            (FieldMetadataType::Boolean, "boolean"),
            (FieldMetadataType::DateTime, "timestamptz"),
            (FieldMetadataType::Date, "date"),
            (FieldMetadataType::Rating, "enum"),
            (FieldMetadataType::Select, "enum"),
            (FieldMetadataType::MultiSelect, "enum"),
            (FieldMetadataType::Files, "jsonb"),
            (FieldMetadataType::RawJson, "jsonb"),
            (FieldMetadataType::TsVector, "tsvector"),
        ];

        for (field_type, column_type) in vectors {
            assert_eq!(
                field_metadata_type_to_column_type(field_type).unwrap(),
                column_type,
                "{field_type:?}"
            );
        }

        let error = field_metadata_type_to_column_type(FieldMetadataType::Actor).unwrap_err();
        assert_eq!(error.code(), "UNSUPPORTED_FIELD_METADATA_TYPE");
        assert_eq!(error.to_string(), "Cannot convert ACTOR to column type.");

        let error = field_metadata_type_to_column_type(FieldMetadataType::Relation).unwrap_err();
        assert_eq!(error.code(), "UNSUPPORTED_FIELD_METADATA_TYPE");
    }

    #[test]
    fn raw_text_column_type_accepts_legacy_rich_text_strings() {
        assert!(is_text_column_type_raw("TEXT"));
        assert!(is_text_column_type_raw("ARRAY"));
        assert!(is_text_column_type_raw("RICH_TEXT"));
        assert!(is_text_column_type_raw("RICH_TEXT_V2"));
        assert!(!is_text_column_type_raw("UUID"));
    }

    #[test]
    fn serialize_function_defaults_map_to_postgres_functions() {
        assert_eq!(
            serialize_default_value(Some(&json!("uuid")), "uuid", SCHEMA, "person", "id").unwrap(),
            "public.uuid_generate_v4()"
        );
        assert_eq!(
            serialize_default_value(
                Some(&json!("now")),
                "timestamptz",
                SCHEMA,
                "person",
                "createdAt"
            )
            .unwrap(),
            "now()"
        );
    }

    #[test]
    fn serialize_requires_single_quoted_strings() {
        let error = serialize_default_value(Some(&json!("NEW")), "text", SCHEMA, "person", "name")
            .unwrap_err();
        assert_eq!(error.code(), "INVALID_FIELD_INPUT");
        assert_eq!(
            error.to_string(),
            "Invalid string default value \"NEW\" should be single quoted"
        );

        assert_eq!(
            serialize_default_value(Some(&json!("'NEW'")), "text", SCHEMA, "person", "name")
                .unwrap(),
            "'NEW'::text"
        );
    }

    #[test]
    fn serialize_enum_defaults_cast_to_the_schema_qualified_enum() {
        assert_eq!(
            serialize_default_value(
                Some(&json!("'NEW'")),
                "enum",
                "workspace_x",
                "opportunity",
                "stage"
            )
            .unwrap(),
            "'NEW'::\"workspace_x\".\"opportunity_stage_enum\""
        );
    }

    #[test]
    fn serialize_boolean_number_and_null_defaults() {
        assert_eq!(
            serialize_default_value(Some(&json!(true)), "boolean", SCHEMA, "person", "isActive")
                .unwrap(),
            "'true'::boolean"
        );
        assert_eq!(
            serialize_default_value(Some(&json!(7)), "float", SCHEMA, "person", "position")
                .unwrap(),
            "'7'::float"
        );
        assert_eq!(
            serialize_default_value(None, "text", SCHEMA, "person", "name").unwrap(),
            "NULL"
        );
        assert_eq!(
            serialize_default_value(Some(&Value::Null), "text", SCHEMA, "person", "name").unwrap(),
            "NULL"
        );
    }

    #[test]
    fn serialize_array_defaults_with_the_cast_array_suffix() {
        assert_eq!(
            serialize_default_value(
                Some(&json!(["'URGENT'", "'LOW_PRIORITY'"])),
                "enum",
                "workspace_x",
                "_person",
                "tags"
            )
            .unwrap(),
            "ARRAY['URGENT','LOW_PRIORITY']::\"workspace_x\".\"_person_tags_enum\"[]"
        );
    }

    #[test]
    fn serialize_object_defaults_as_json_strings() {
        assert_eq!(
            serialize_default_value(
                Some(&json!({"key": "value"})),
                "jsonb",
                SCHEMA,
                "person",
                "properties"
            )
            .unwrap(),
            "'{\"key\":\"value\"}'::jsonb"
        );
    }

    #[test]
    fn build_emits_the_definition_in_the_upstream_part_order() {
        let definition = WorkspaceSchemaColumnDefinition {
            name: "id".to_string(),
            column_type: "uuid".to_string(),
            is_nullable: Some(false),
            default: Some("public.uuid_generate_v4()".to_string()),
            is_primary: true,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        };

        assert_eq!(
            build_sql_column_definition(&definition).unwrap(),
            "\"id\" uuid PRIMARY KEY NOT NULL DEFAULT public.uuid_generate_v4()"
        );

        let identity_column = WorkspaceSchemaColumnDefinition {
            name: "compactId".to_string(),
            column_type: "integer".to_string(),
            is_nullable: Some(false),
            default: None,
            is_primary: false,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: true,
        };

        assert_eq!(
            build_sql_column_definition(&identity_column).unwrap(),
            "\"compactId\" integer GENERATED ALWAYS AS IDENTITY NOT NULL"
        );
    }

    #[test]
    fn build_emits_generated_stored_columns_for_tsvector_and_rejects_unsafe_expressions() {
        let definition = WorkspaceSchemaColumnDefinition {
            name: "searchVector".to_string(),
            column_type: "tsvector".to_string(),
            is_nullable: Some(true),
            default: Some("NULL".to_string()),
            is_primary: false,
            is_array: false,
            as_expression: Some("to_tsvector('simple', COALESCE(\"jobTitle\", ''))".to_string()),
            generated_type: Some(GeneratedType::Stored),
            is_generated_always_as_identity: false,
        };

        assert_eq!(
            build_sql_column_definition(&definition).unwrap(),
            "\"searchVector\" tsvector GENERATED ALWAYS AS (to_tsvector('simple', COALESCE(\"jobTitle\", ''))) STORED"
        );

        let unsafe_definition = WorkspaceSchemaColumnDefinition {
            as_expression: Some("to_tsvector('simple', \"a\"); DROP TABLE x".to_string()),
            ..definition
        };

        assert_eq!(
            build_sql_column_definition(&unsafe_definition).unwrap_err(),
            ColumnDefsError::UnsafeTsVectorExpression
        );
    }

    #[test]
    fn build_emits_array_types_and_enum_types() {
        let definition = WorkspaceSchemaColumnDefinition {
            name: "tags".to_string(),
            column_type: format!("\"{SCHEMA}\".\"_person_tags_enum\""),
            is_nullable: Some(true),
            default: Some("NULL".to_string()),
            is_primary: false,
            is_array: true,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        };

        assert_eq!(
            build_sql_column_definition(&definition).unwrap(),
            format!("\"tags\" \"{SCHEMA}\".\"_person_tags_enum\"[] DEFAULT NULL")
        );
    }

    #[test]
    fn scalar_text_field_generates_a_nullable_column_with_null_default() {
        let columns = generate_column_definitions(
            &field("description", FieldMetadataType::Text),
            SCHEMA,
            "_person",
            None,
        )
        .unwrap();

        assert_eq!(
            columns,
            vec![WorkspaceSchemaColumnDefinition {
                name: "description".to_string(),
                column_type: "text".to_string(),
                is_nullable: None,
                default: Some("NULL".to_string()),
                is_primary: false,
                is_array: false,
                as_expression: None,
                generated_type: None,
                is_generated_always_as_identity: false,
            }]
        );
    }

    #[test]
    fn boolean_field_serializes_the_true_default() {
        let mut input = field("isActive", FieldMetadataType::Boolean);
        input.default_value = Some(json!(true));

        let columns = generate_column_definitions(&input, SCHEMA, "_person", None).unwrap();

        assert_eq!(columns[0].column_type, "boolean");
        assert_eq!(columns[0].default, Some("'true'::boolean".to_string()));
    }

    #[test]
    fn enum_fields_use_the_schema_qualified_enum_name_and_array_flag_for_multi_select() {
        let select_field = field("status", FieldMetadataType::Select);

        let columns = generate_column_definitions(&select_field, SCHEMA, "_person", None).unwrap();

        assert_eq!(columns.len(), 1);
        assert_eq!(
            columns[0].column_type,
            format!("\"{SCHEMA}\".\"_person_status_enum\"")
        );
        assert_eq!(columns[0].default, Some("NULL".to_string()));
        assert!(!columns[0].is_array);

        let multi_select_field = field("tags", FieldMetadataType::MultiSelect);

        let columns =
            generate_column_definitions(&multi_select_field, SCHEMA, "_person", None).unwrap();

        assert_eq!(
            columns[0].column_type,
            format!("\"{SCHEMA}\".\"_person_tags_enum\"")
        );
        assert!(columns[0].is_array);
    }

    #[test]
    fn many_to_one_relations_generate_a_nullable_uuid_join_column() {
        let mut relation = field("company", FieldMetadataType::Relation);
        relation.relation_type = Some(MANY_TO_ONE_RELATION_TYPE.to_string());

        let columns = generate_column_definitions(&relation, SCHEMA, "_person", None).unwrap();

        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].name, "companyId");
        assert_eq!(columns[0].column_type, "uuid");
        assert_eq!(columns[0].default, None);
        assert!(!columns[0].is_primary);

        let mut one_to_many = relation.clone();
        one_to_many.relation_type = Some("ONE_TO_MANY".to_string());

        assert!(
            generate_column_definitions(&one_to_many, SCHEMA, "_person", None)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn morph_relation_heads_generate_zero_columns() {
        let mut morph_head = field("attachments", FieldMetadataType::MorphRelation);
        morph_head.morph_id = Some(morph_head.universal_identifier.clone());

        assert!(
            generate_column_definitions(&morph_head, SCHEMA, "_person", None)
                .unwrap()
                .is_empty()
        );

        let mut morph_side = morph_head.clone();
        morph_side.morph_id = Some("other-id".to_string());
        morph_side.relation_type = Some(MANY_TO_ONE_RELATION_TYPE.to_string());

        let columns = generate_column_definitions(&morph_side, SCHEMA, "_person", None).unwrap();
        assert_eq!(columns[0].name, "attachmentsId");
    }

    #[test]
    fn address_composite_generates_the_eight_flattened_columns() {
        let columns = generate_column_definitions(
            &field("homeAddress", FieldMetadataType::Address),
            SCHEMA,
            "_person",
            None,
        )
        .unwrap();

        let names: Vec<&str> = columns.iter().map(|column| column.name.as_str()).collect();

        assert_eq!(
            names,
            vec![
                "homeAddressAddressStreet1",
                "homeAddressAddressStreet2",
                "homeAddressAddressCity",
                "homeAddressAddressPostcode",
                "homeAddressAddressState",
                "homeAddressAddressCountry",
                "homeAddressAddressLat",
                "homeAddressAddressLng"
            ]
        );

        for column in &columns {
            assert_eq!(column.is_nullable, Some(true));
            assert!(!column.is_primary);
            assert_eq!(column.default, Some("NULL".to_string()));
        }

        assert_eq!(columns[6].column_type, "numeric");
        assert_eq!(columns[7].column_type, "numeric");
    }

    #[test]
    fn currency_composite_serializes_its_defaults_with_casts() {
        let mut price = field("price", FieldMetadataType::Currency);
        price.default_value = Some(json!({
            "amountMicros": "'100000000'",
            "currencyCode": "'USD'"
        }));

        let columns = generate_column_definitions(&price, SCHEMA, "_person", None).unwrap();

        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].name, "priceAmountMicros");
        assert_eq!(columns[0].column_type, "numeric");
        assert_eq!(columns[0].default, Some("'100000000'::numeric".to_string()));
        assert_eq!(columns[1].name, "priceCurrencyCode");
        assert_eq!(columns[1].column_type, "text");
        assert_eq!(columns[1].default, Some("'USD'::text".to_string()));
    }

    #[test]
    fn phones_composite_nullifies_empty_defaults() {
        let mut phone = field("phone", FieldMetadataType::Phones);
        phone.default_value = Some(json!({
            "primaryPhoneNumber": "''",
            "primaryPhoneCountryCode": "'US'",
            "primaryPhoneCallingCode": "'+1'",
            "additionalPhones": Value::Null
        }));

        let columns = generate_column_definitions(&phone, SCHEMA, "_person", None).unwrap();

        assert_eq!(columns.len(), 4);
        assert_eq!(columns[0].name, "phonePrimaryPhoneNumber");
        assert_eq!(columns[0].default, Some("NULL".to_string()));
        assert_eq!(columns[1].default, Some("'US'::text".to_string()));
        assert_eq!(columns[2].default, Some("'+1'::text".to_string()));
        assert_eq!(columns[3].name, "phoneAdditionalPhones");
        assert_eq!(columns[3].column_type, "jsonb");
    }

    #[test]
    fn actor_composite_select_source_gets_its_own_enum_type() {
        let mut actor = field("createdBy", FieldMetadataType::Actor);
        actor.default_value = Some(json!({
            "source": "'MANUAL'",
            "workspaceMemberId": Value::Null,
            "name": "'System'"
        }));

        let columns = generate_column_definitions(&actor, SCHEMA, "_person", None).unwrap();

        assert_eq!(
            columns
                .iter()
                .map(|column| column.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "createdBySource",
                "createdByWorkspaceMemberId",
                "createdByName",
                "createdByContext"
            ]
        );
        assert_eq!(
            columns[0].column_type,
            format!("\"{SCHEMA}\".\"_person_createdBySource_enum\"")
        );
        assert_eq!(
            columns[0].default,
            Some(format!(
                "'MANUAL'::\"{SCHEMA}\".\"_person_createdBySource_enum\""
            ))
        );
        assert_eq!(columns[1].column_type, "uuid");
        assert_eq!(columns[1].default, Some("NULL".to_string()));
        assert_eq!(columns[2].default, Some("'System'::text".to_string()));
        assert_eq!(columns[3].column_type, "jsonb");
    }

    #[test]
    fn rich_text_composite_adds_the_server_only_file_ids_column() {
        let columns = generate_column_definitions(
            &field("bodyV2", FieldMetadataType::RichText),
            SCHEMA,
            "_person",
            None,
        )
        .unwrap();

        assert_eq!(
            columns
                .iter()
                .map(|column| column.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "bodyV2Blocknote",
                "bodyV2Markdown",
                "bodyV2Tiptap",
                "bodyV2_fileIds"
            ]
        );
        assert_eq!(columns[3].column_type, "jsonb");
        assert_eq!(columns[3].default, None);
    }

    #[test]
    fn tsvector_column_uses_the_search_expression_and_stays_nullable() {
        let columns = generate_column_definitions(
            &field("searchVector", FieldMetadataType::TsVector),
            SCHEMA,
            "_person",
            Some("to_tsvector('simple', COALESCE(\"jobTitle\", ''))"),
        )
        .unwrap();

        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].column_type, "tsvector");
        assert_eq!(columns[0].default, None);
        assert_eq!(
            columns[0].as_expression,
            Some("to_tsvector('simple', COALESCE(\"jobTitle\", ''))".to_string())
        );
        assert_eq!(columns[0].generated_type, Some(GeneratedType::Stored));
    }

    #[test]
    fn composite_relation_properties_are_rejected() {
        let mut parent = field("weird", FieldMetadataType::FullName);
        parent.default_value = None;

        let property = composite_types::CompositeProperty {
            name: "relation",
            field_type: FieldMetadataType::Relation,
            is_included_in_unique_constraint: false,
            is_array: false,
            options: &[],
        };

        let error = generate_composite_column_definition(&property, &parent, SCHEMA, "_person")
            .unwrap_err();

        assert_eq!(error.code(), "UNSUPPORTED_COMPOSITE_COLUMN_TYPE");
    }

    #[test]
    fn composite_type_registry_covers_the_eight_upstream_types() {
        assert_eq!(COMPOSITE_TYPES.len(), 8);
    }
}
