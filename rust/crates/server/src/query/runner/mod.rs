use serde_json::Value;

use crate::query::cursor::CursorError;
use crate::query::filter::model::FilterError;
use crate::query::order::OrderError;
use crate::query::repository::table_shape::ColumnShape;
use crate::query::repository::table_shape::TableShape;

pub mod actors;
pub mod aggregate;
pub mod base;
pub mod create;
pub mod find_many;
pub mod find_one;
pub mod group_by;
pub mod mutation;
pub mod nested;
pub mod permissions;
pub mod position;
pub mod rls;
#[cfg(test)]
pub(crate) mod test_support;

pub use actors::ActorContext;
pub use actors::ActorSource;
pub use aggregate::aggregate;
pub use aggregate::aggregate_expression;
pub use aggregate::aggregate_result_name;
pub use aggregate::Aggregate;
pub use aggregate::AggregateOperation;
pub use aggregate::AggregateValue;
pub use base::RunnerSchema;
pub use base::COMMON_QUERY_COMPLEXITY_LIMIT;
pub use base::DEFAULT_PAGE_SIZE;
pub use base::NESTED_RELATION_QUERY_MAX_CONCURRENCY;
pub use base::QUERY_MAX_RECORDS;
pub use base::QUERY_MAX_RECORDS_FROM_RELATION;
pub use create::create_many;
pub use create::create_one;
pub use create::CreateArgs;
pub use find_many::find_many;
pub use find_many::FindManyArgs;
pub use find_many::FindManyResult;
pub use find_many::NestedRelationSelection;
pub use find_one::find_one;
pub use group_by::group_by;
pub use group_by::group_expression;
pub use group_by::Granularity;
pub use group_by::GroupByDefinition;
pub use group_by::GroupByRow;
pub use group_by::DEFAULT_NUMBER_OF_GROUPS_LIMIT;
pub use mutation::require_filtered_mutation;
pub use mutation::run_mutation;
pub use mutation::run_mutation_one;
pub use mutation::FilteredMutationKind;

#[derive(Debug, Clone, PartialEq)]
pub enum RunnerError {
    Code { code: &'static str, message: String },
    Database(String),
}

impl RunnerError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self::Code {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Code { code, .. } => code,
            Self::Database(_) => "INTERNAL_SERVER_ERROR",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Code { message, .. } => message,
            Self::Database(message) => message,
        }
    }

    pub fn unknown_object(name_singular: &str) -> Self {
        Self::new(
            "UNKNOWN_OBJECT",
            format!("Object \"{name_singular}\" does not exist in this workspace"),
        )
    }

    pub fn unknown_relation(field_name: &str) -> Self {
        Self::new(
            "UNKNOWN_RELATION",
            format!("Unknown relation \"{field_name}\""),
        )
    }

    pub fn unknown_column(column_name: &str, object_name: &str) -> Self {
        Self::new(
            "UNKNOWN_COLUMN",
            format!("Column \"{column_name}\" does not exist on \"{object_name}\""),
        )
    }

    pub fn unknown_sub_field(field_name: &str, sub_field: &str, available: Vec<String>) -> Self {
        Self::new(
            "UNKNOWN_SUB_FIELD",
            format!(
                "Sub field \"{sub_field}\" not found for composite field \"{field_name}\". Available sub fields: {}.",
                available.join(", ")
            ),
        )
    }

    pub fn invalid_query(message: impl Into<String>) -> Self {
        Self::new("INVALID_QUERY_INPUT", message)
    }

    pub fn invalid_direction(direction: &str) -> Self {
        Self::new(
            "INVALID_DIRECTION",
            format!("Invalid direction: {direction}"),
        )
    }

    pub fn invalid_parameter() -> Self {
        Self::new("INVALID_PARAMETER", "A query parameter could not be bound")
    }

    pub fn args_conflict(left: &str, right: &str) -> Self {
        Self::new(
            "ARGS_CONFLICT",
            format!("You can either use {left} or {right}, but not both at the same time"),
        )
    }

    pub fn invalid_page_argument() -> Self {
        Self::new(
            "INVALID_ARGS",
            "first and last must be non-negative integers below the query limit",
        )
    }

    pub fn too_complex() -> Self {
        Self::new(
            "TOO_COMPLEX_QUERY",
            format!("The query exceeds the maximum complexity of {COMMON_QUERY_COMPLEXITY_LIMIT}"),
        )
    }

    pub fn record_not_found() -> Self {
        Self::new("RECORD_NOT_FOUND", "Record was not found")
    }

    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new("PERMISSION_DENIED", message)
    }

    pub fn record_not_editable() -> Self {
        Self::new("RECORD_NOT_EDITABLE", "Record is not editable")
    }

    pub fn too_many_records_to_update() -> Self {
        Self::new(
            "TOO_MANY_RECORDS_TO_UPDATE",
            format!("Cannot update more than {QUERY_MAX_RECORDS} records at once"),
        )
    }

    pub fn missing_filter() -> Self {
        Self::new("INVALID_QUERY_INPUT", "Missing filter argument")
    }

    pub fn from_filter(error: FilterError) -> Self {
        Self::new(error.code(), error.message())
    }

    pub fn from_repository(error: crate::query::repository::RepositoryError) -> Self {
        Self::new(error.code(), error.message())
    }

    pub fn from_cursor(error: CursorError) -> Self {
        Self::new(error.code(), error.message())
    }

    pub fn from_order(error: OrderError) -> Self {
        Self::new(error.code(), error.message())
    }

    pub fn database(error: impl std::fmt::Display) -> Self {
        Self::Database(error.to_string())
    }
}

impl From<crate::query::repository::RepositoryError> for RunnerError {
    fn from(error: crate::query::repository::RepositoryError) -> Self {
        Self::new(error.code(), error.message())
    }
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.message())
    }
}

impl std::error::Error for RunnerError {}

pub fn format_result_rows(
    model: &base::ObjectModel,
    columns: &[(ColumnShape, String, String)],
    rows: &[serde_json::Map<String, Value>],
) -> Result<Vec<Value>, RunnerError> {
    let mut records = Vec::with_capacity(rows.len());

    for row in rows {
        let mut record = serde_json::Map::new();
        let mut composites = serde_json::Map::new();

        for (column, result_alias, _) in columns {
            let value = row.get(result_alias).cloned().ok_or_else(|| {
                RunnerError::unknown_column(result_alias, &model.object.name_singular)
            })?;
            let value = normalize_null_equivalent(column, &model.shape, value);

            if let Some(parent) = column.composite_parent_field_name.as_deref() {
                let suffix = column
                    .column_name
                    .strip_prefix(parent)
                    .unwrap_or(&column.column_name);
                let value = if value.is_null() && column.is_array {
                    Value::Array(Vec::new())
                } else {
                    value
                };
                let entry = composites
                    .entry(parent.to_string())
                    .or_insert_with(|| Value::Object(serde_json::Map::new()));
                let object = entry.as_object_mut().expect("composite container");
                object.insert(base::lowercase_first(suffix), value);
            } else {
                record.insert(column.field_name.clone(), value);
            }
        }

        for (field_name, value) in composites {
            record.insert(field_name, value);
        }

        records.push(Value::Object(record));
    }

    Ok(records)
}

fn normalize_null_equivalent(column: &ColumnShape, shape: &TableShape, value: Value) -> Value {
    if value != Value::String(String::new()) && !value.is_null() {
        return value;
    }

    match column.field_metadata_type {
        crate::metadata::flat::FieldMetadataType::Text if column.empty_string_default => {
            Value::String(String::new())
        }
        crate::metadata::flat::FieldMetadataType::MultiSelect => {
            let field_name = column
                .composite_parent_field_name
                .clone()
                .unwrap_or_else(|| column.field_name.clone());

            if shape.column(&field_name).is_some() {
                value
            } else {
                Value::Null
            }
        }
        _ => value,
    }
}
