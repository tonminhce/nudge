use std::fmt;

pub mod sql;
pub mod table_shape;

pub use sql::build_column_result_alias;
pub use sql::build_delete_statement;
pub use sql::build_insert_execution;
pub use sql::build_insert_rows;
pub use sql::build_insert_statement;
pub use sql::build_mutation_statement;
pub use sql::build_restore_statement;
pub use sql::build_soft_delete_statement;
pub use sql::build_update_statement;
pub use sql::cast_named_parameters;
pub use sql::cast_set_parameters;
pub use sql::check_update_many_limit;
pub use sql::expand_record_columns;
pub use sql::quote_identifier;
pub use sql::Conjunction;
pub use sql::InsertExecution;
pub use sql::InsertRowValue;
pub use sql::InsertRows;
pub use sql::MutationKind;
pub use sql::MutationStatement;
pub use sql::MutationStatementState;
pub use sql::MutationWhereClause;
pub use sql::INSERT_PARAMETER_PREFIX;
pub use sql::QUERY_MAX_RECORDS;
pub use sql::SET_PARAMETER_PREFIX;
pub use table_shape::build_workspace_table_shape;
pub use table_shape::ColumnShape;
pub use table_shape::RelationShape;
pub use table_shape::TableShape;
pub use table_shape::TableShapeCache;
pub use table_shape::TableShapeKey;
pub use table_shape::DEFAULT_TABLE_SHAPE_CACHE_CAPACITY;

#[derive(Debug, Clone, PartialEq)]
pub struct RepositoryError {
    code: &'static str,
    message: String,
}

impl RepositoryError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn unknown_object(name_singular: &str) -> Self {
        Self::new(
            "UNKNOWN_OBJECT",
            format!("Object \"{name_singular}\" does not exist in this workspace"),
        )
    }

    pub fn unknown_column(column_name: &str, table: &str) -> Self {
        Self::new(
            "UNKNOWN_COLUMN",
            format!("Column \"{column_name}\" does not exist on \"{table}\""),
        )
    }

    pub fn unsupported_operation(message: impl Into<String>) -> Self {
        Self::new("UNSUPPORTED_OPERATION", message)
    }

    pub fn invalid_query(message: impl Into<String>) -> Self {
        Self::new("INVALID_QUERY", message)
    }

    pub fn too_many_records_to_update() -> Self {
        Self::new(
            "TOO_MANY_RECORDS_TO_UPDATE",
            format!("Cannot update more than {QUERY_MAX_RECORDS} records at once"),
        )
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for RepositoryError {}
