use serde_json::Map;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;

use crate::metadata::flat::FieldMetadataType;
use crate::query::compile::compile_named_parameters;
use crate::query::compile::CompiledStatement;
use crate::workspace_migrations::ddl::escape::escape_identifier;

use super::table_shape::ColumnShape;
use super::table_shape::TableShape;
use super::RepositoryError;

pub const QUERY_MAX_RECORDS: usize = 200;

pub const INSERT_PARAMETER_PREFIX: &str = "ormInsert_";

pub const SET_PARAMETER_PREFIX: &str = "ormSet_";

pub const ACTOR_SOURCE_ENUM_COLUMNS: [&str; 2] = ["createdBySource", "updatedBySource"];

pub const WORKSPACE_MEMBER_ID_COLUMN_SUFFIX: &str = "WorkspaceMemberId";

const UPDATED_AT_COLUMN_NAME: &str = "updatedAt";

const DELETED_AT_COLUMN_NAME: &str = "deletedAt";

const POSTGRES_MAX_ALIAS_LENGTH: usize = 63;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationKind {
    Update,
    Delete,
    SoftDelete,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conjunction {
    And,
    Or,
}

impl Conjunction {
    pub fn keyword(self) -> &'static str {
        match self {
            Conjunction::And => "AND",
            Conjunction::Or => "OR",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationWhereClause {
    pub conjunction: Conjunction,
    pub sql: String,
}

impl MutationWhereClause {
    pub fn and(sql: impl Into<String>) -> Self {
        Self {
            conjunction: Conjunction::And,
            sql: sql.into(),
        }
    }

    pub fn or(sql: impl Into<String>) -> Self {
        Self {
            conjunction: Conjunction::Or,
            sql: sql.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsertRowValue {
    Parameter(String),
    Default,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InsertRows {
    pub column_names: Vec<String>,
    pub rows: Vec<Vec<InsertRowValue>>,
    pub parameters: Map<String, Value>,
    pub inserted_columns: Vec<String>,
}

pub fn quote_identifier(identifier: &str) -> Result<String, RepositoryError> {
    escape_identifier(identifier).map_err(|error| RepositoryError::invalid_query(error.message))
}

pub fn build_column_result_alias(alias: &str, column_name: &str) -> String {
    let candidate = format!("{alias}_{column_name}");

    if candidate.len() <= POSTGRES_MAX_ALIAS_LENGTH {
        return candidate;
    }

    let digest = Sha256::digest(candidate.as_bytes());
    let hex = format!("{digest:064x}");

    hex[..POSTGRES_MAX_ALIAS_LENGTH].to_string()
}

fn decapitalize(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + characters.as_str(),
    }
}

fn serialize_jsonb_write_value(column: Option<&ColumnShape>, value: &Value) -> Value {
    let Some(column) = column else {
        return value.clone();
    };

    match column.field_metadata_type {
        FieldMetadataType::RawJson => match value {
            Value::Null => Value::Null,
            Value::String(raw) => match serde_json::from_str::<Value>(raw) {
                Ok(parsed) => Value::String(parsed.to_string()),
                Err(_) => Value::String(raw.clone()),
            },
            other => Value::String(other.to_string()),
        },
        FieldMetadataType::Files => match value {
            Value::Null => Value::Null,
            other => Value::String(other.to_string()),
        },
        _ => value.clone(),
    }
}

fn composite_parent_field(shape: &TableShape, key: &str) -> bool {
    shape.columns.iter().any(|column| {
        column
            .composite_parent_field_name
            .as_deref()
            .is_some_and(|parent| parent == key)
    })
}

pub fn expand_record_columns(
    shape: &TableShape,
    data: &Map<String, Value>,
) -> Result<Vec<(String, Value)>, RepositoryError> {
    let mut expanded: Vec<(String, Value)> = Vec::new();

    for (key, value) in data {
        if shape.column(key).is_some() {
            expanded.push((key.clone(), value.clone()));
            continue;
        }

        if composite_parent_field(shape, key) {
            let Some(object) = value.as_object() else {
                if value.is_null() {
                    continue;
                }

                return Err(RepositoryError::unknown_column(key, &shape.name_singular));
            };

            for column in &shape.columns {
                let Some(parent) = &column.composite_parent_field_name else {
                    continue;
                };

                if parent != key {
                    continue;
                }

                let suffix = column
                    .column_name
                    .strip_prefix(parent.as_str())
                    .unwrap_or(&column.column_name);

                if let Some(sub_value) = object.get(&decapitalize(suffix)) {
                    expanded.push((column.column_name.clone(), sub_value.clone()));
                }
            }

            continue;
        }

        return Err(RepositoryError::unknown_column(key, &shape.name_singular));
    }

    Ok(expanded)
}

pub fn build_insert_rows(
    shape: &TableShape,
    records: &[Map<String, Value>],
) -> Result<InsertRows, RepositoryError> {
    let mut formatted: Vec<Vec<(String, Value)>> = Vec::with_capacity(records.len());
    let mut column_names: Vec<String> = Vec::new();

    for record in records {
        let expanded = expand_record_columns(shape, record)?;

        for (column_name, _) in &expanded {
            if !column_names.iter().any(|existing| existing == column_name) {
                column_names.push(column_name.clone());
            }
        }

        formatted.push(expanded);
    }

    let mut parameters: Map<String, Value> = Map::new();
    let mut parameter_sequence: usize = 0;
    let mut rows: Vec<Vec<InsertRowValue>> = Vec::with_capacity(formatted.len());

    for expanded in &formatted {
        let mut row: Vec<InsertRowValue> = Vec::with_capacity(column_names.len());

        for column_name in &column_names {
            match expanded.iter().find(|(name, _)| name == column_name) {
                None => row.push(InsertRowValue::Default),
                Some((_, value)) => {
                    let parameter_name = format!("{INSERT_PARAMETER_PREFIX}{parameter_sequence}");
                    parameter_sequence += 1;

                    parameters.insert(
                        parameter_name.clone(),
                        serialize_jsonb_write_value(shape.column(column_name), value),
                    );
                    row.push(InsertRowValue::Parameter(parameter_name));
                }
            }
        }

        rows.push(row);
    }

    let inserted_columns = column_names.clone();

    Ok(InsertRows {
        column_names,
        rows,
        parameters,
        inserted_columns,
    })
}

pub fn build_insert_statement(
    shape: &TableShape,
    column_names: &[String],
    rows: &[Vec<InsertRowValue>],
    returning_columns: &[String],
    on_conflict_do_nothing: bool,
) -> Result<String, RepositoryError> {
    if column_names.is_empty() || rows.is_empty() {
        return Err(RepositoryError::invalid_query(format!(
            "An INSERT on \"{}\" needs at least one column and one row",
            shape.name_singular
        )));
    }

    let mut column_list: Vec<String> = Vec::with_capacity(column_names.len());
    for column_name in column_names {
        column_list.push(quote_identifier(column_name)?);
    }

    let mut value_tuples: Vec<String> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut rendered: Vec<String> = Vec::with_capacity(row.len());

        for value in row {
            rendered.push(match value {
                InsertRowValue::Default => "DEFAULT".to_string(),
                InsertRowValue::Parameter(name) => format!(":{name}"),
            });
        }

        value_tuples.push(format!("({})", rendered.join(", ")));
    }

    let mut parts: Vec<String> = Vec::with_capacity(4);
    parts.push(format!(
        "INSERT INTO {}.{} ({})",
        quote_identifier(&shape.schema_name)?,
        quote_identifier(&shape.table_name)?,
        column_list.join(", ")
    ));
    parts.push(format!("VALUES {}", value_tuples.join(", ")));

    if on_conflict_do_nothing {
        parts.push("ON CONFLICT DO NOTHING".to_string());
    }

    if !returning_columns.is_empty() {
        let mut returning: Vec<String> = Vec::with_capacity(returning_columns.len());
        for column_name in returning_columns {
            returning.push(quote_identifier(column_name)?);
        }
        parts.push(format!("RETURNING {}", returning.join(", ")));
    }

    Ok(parts.join(" "))
}

pub fn cast_named_parameters(
    sql: &str,
    suffix_for: &dyn Fn(&str) -> Result<Option<String>, RepositoryError>,
) -> Result<(String, Vec<String>), RepositoryError> {
    let characters: Vec<char> = sql.chars().collect();
    let mut rendered = String::new();
    let mut matched: Vec<String> = Vec::new();
    let mut index = 0usize;

    while index < characters.len() {
        let character = characters[index];

        if character == '\'' || character == '"' {
            let mut closing_index = index + 1;

            while closing_index < characters.len() {
                if characters[closing_index] != character {
                    closing_index += 1;
                    continue;
                }

                if characters.get(closing_index + 1) == Some(&character) {
                    closing_index += 2;
                    continue;
                }

                break;
            }

            if closing_index >= characters.len() {
                return Err(RepositoryError::invalid_query(format!(
                    "Unterminated {character} quoted token in SQL"
                )));
            }

            rendered.extend(characters[index..=closing_index].iter());
            index = closing_index + 1;
            continue;
        }

        if character != ':' {
            rendered.push(character);
            index += 1;
            continue;
        }

        if characters.get(index + 1) == Some(&':') {
            rendered.push_str("::");
            index += 2;
            continue;
        }

        let mut name_end_index = index + 1;
        while name_end_index < characters.len()
            && (characters[name_end_index].is_ascii_alphanumeric()
                || characters[name_end_index] == '_')
        {
            name_end_index += 1;
        }

        let parameter_name: String = characters[index + 1..name_end_index].iter().collect();

        if parameter_name.is_empty() {
            rendered.push(':');
            index += 1;
            continue;
        }

        match suffix_for(parameter_name.as_str())? {
            None => rendered.extend([':'].into_iter().chain(parameter_name.chars())),
            Some(suffix) => {
                rendered.extend(
                    [':']
                        .into_iter()
                        .chain(parameter_name.chars())
                        .chain(suffix.chars()),
                );
                matched.push(parameter_name);
            }
        }

        index = name_end_index;
    }

    Ok((rendered, matched))
}

#[derive(Debug, Clone, PartialEq)]
pub struct InsertExecution {
    pub sql: String,
    pub columns: Vec<String>,
}

pub fn build_insert_execution(
    shape: &TableShape,
    rows: &InsertRows,
    returning_columns: &[String],
    on_conflict_do_nothing: bool,
) -> Result<InsertExecution, RepositoryError> {
    let statement = build_insert_statement(
        shape,
        &rows.column_names,
        &rows.rows,
        returning_columns,
        on_conflict_do_nothing,
    )?;
    let mut column_by_parameter: HashMap<String, String> = HashMap::new();

    for row in &rows.rows {
        for (column_name, value) in rows.column_names.iter().zip(row.iter()) {
            if let InsertRowValue::Parameter(parameter_name) = value {
                column_by_parameter.insert(parameter_name.clone(), column_name.clone());
            }
        }
    }

    let (sql, matched) = cast_named_parameters(&statement, &|parameter_name| {
        let Some(column_name) = column_by_parameter.get(parameter_name) else {
            return Ok(None);
        };

        Ok(Some(insert_column_cast(shape, column_name)?))
    })?;
    let columns = matched
        .iter()
        .filter_map(|parameter_name| column_by_parameter.get(parameter_name).cloned())
        .collect::<Vec<_>>();

    Ok(InsertExecution { sql, columns })
}

pub fn insert_column_cast(
    shape: &TableShape,
    column_name: &str,
) -> Result<String, RepositoryError> {
    if ACTOR_SOURCE_ENUM_COLUMNS.contains(&column_name) {
        let enum_name =
            crate::metadata::names::compute_postgres_enum_name(&shape.table_name, column_name);

        return Ok(format!(
            "::text::{}.{}",
            quote_identifier(&shape.schema_name)?,
            quote_identifier(&enum_name)?
        ));
    }

    if column_name.ends_with(WORKSPACE_MEMBER_ID_COLUMN_SUFFIX) {
        return Ok(String::from("::uuid"));
    }

    if shape.column(column_name).is_none() {
        return Ok(String::new());
    }

    set_column_cast(shape, column_name)
}

pub fn set_column_cast(shape: &TableShape, column_name: &str) -> Result<String, RepositoryError> {
    let column = shape
        .column(column_name)
        .ok_or_else(|| RepositoryError::unknown_column(column_name, &shape.name_singular))?;

    Ok(match column.field_metadata_type {
        FieldMetadataType::Uuid
        | FieldMetadataType::Relation
        | FieldMetadataType::MorphRelation => String::from("::uuid"),
        FieldMetadataType::RawJson => String::from("::jsonb"),
        FieldMetadataType::DateTime => String::from("::timestamptz"),
        FieldMetadataType::Date => String::from("::date"),
        FieldMetadataType::Select if ACTOR_SOURCE_ENUM_COLUMNS.contains(&column_name) => {
            let enum_name =
                crate::metadata::names::compute_postgres_enum_name(&shape.table_name, column_name);

            format!(
                "::text::{}.{}",
                quote_identifier(&shape.schema_name)?,
                quote_identifier(&enum_name)?
            )
        }
        _ => String::new(),
    })
}

pub fn cast_set_parameters(
    sql: &str,
    shape: &TableShape,
    set_columns: &[String],
) -> Result<String, RepositoryError> {
    let (sql, _) = cast_named_parameters(sql, &|parameter_name| {
        let Some(suffix) = parameter_name.strip_prefix(SET_PARAMETER_PREFIX) else {
            return Ok(None);
        };
        let Ok(index) = suffix.parse::<usize>() else {
            return Ok(None);
        };
        let Some(column_name) = set_columns.get(index) else {
            return Ok(None);
        };

        Ok(Some(set_column_cast(shape, column_name)?))
    })?;

    Ok(sql)
}

fn render_user_where_expression(
    where_clauses: &[MutationWhereClause],
    row_access_conditions: &[String],
) -> String {
    let mut user_expression = String::new();

    for (index, clause) in where_clauses.iter().enumerate() {
        if index == 0 {
            user_expression.push_str(&clause.sql);
        } else {
            user_expression.push_str(&format!(" {} {}", clause.conjunction.keyword(), clause.sql));
        }
    }

    if row_access_conditions.is_empty() {
        return user_expression;
    }

    let row_access: Vec<String> = row_access_conditions
        .iter()
        .map(|condition| format!("({condition})"))
        .collect();
    let row_access_expression = row_access.join(" AND ");

    if user_expression.is_empty() {
        return row_access_expression;
    }

    format!("({user_expression}) AND {row_access_expression}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationStatementState<'a> {
    pub alias: &'a str,
    pub shape: &'a TableShape,
    pub kind: MutationKind,
    pub set_columns: Vec<(String, Value)>,
    pub where_clauses: Vec<MutationWhereClause>,
    pub where_parameters: Map<String, Value>,
    pub row_access_conditions: Vec<String>,
    pub include_deleted: bool,
    pub returning_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutationStatement {
    pub sql: String,
    pub parameters: Map<String, Value>,
}

impl MutationStatement {
    pub fn compile(&self) -> Result<CompiledStatement, RepositoryError> {
        compile_named_parameters(&self.sql, &self.parameters)
            .map_err(|error| RepositoryError::invalid_query(error.message()))
    }
}

fn with_updated_at_maintenance(
    shape: &TableShape,
    mut clauses: Vec<(String, String)>,
) -> Vec<(String, String)> {
    if shape.column(UPDATED_AT_COLUMN_NAME).is_none() {
        return clauses;
    }

    clauses.push((
        UPDATED_AT_COLUMN_NAME.to_string(),
        "CURRENT_TIMESTAMP".to_string(),
    ));
    clauses
}

fn assert_deleted_at_column(shape: &TableShape) -> Result<(), RepositoryError> {
    if shape.has_deleted_at_column {
        return Ok(());
    }

    Err(RepositoryError::unsupported_operation(format!(
        "\"{}\" has no \"{DELETED_AT_COLUMN_NAME}\" column, so it cannot be soft-deleted or restored",
        shape.name_singular
    )))
}

fn build_set_clauses(
    state: &MutationStatementState<'_>,
    parameters: &mut Map<String, Value>,
) -> Result<Vec<(String, String)>, RepositoryError> {
    match state.kind {
        MutationKind::Delete => Ok(Vec::new()),
        MutationKind::SoftDelete => {
            assert_deleted_at_column(state.shape)?;

            Ok(with_updated_at_maintenance(
                state.shape,
                vec![(
                    DELETED_AT_COLUMN_NAME.to_string(),
                    "CURRENT_TIMESTAMP".to_string(),
                )],
            ))
        }
        MutationKind::Restore => {
            assert_deleted_at_column(state.shape)?;

            Ok(with_updated_at_maintenance(
                state.shape,
                vec![(DELETED_AT_COLUMN_NAME.to_string(), "NULL".to_string())],
            ))
        }
        MutationKind::Update => {
            let mut clauses: Vec<(String, String)> = Vec::new();
            let mut sequence: usize = 0;

            for (column_name, value) in &state.set_columns {
                let column = state.shape.column(column_name).ok_or_else(|| {
                    RepositoryError::unknown_column(column_name, &state.shape.name_singular)
                })?;

                let mut parameter_name = format!("{SET_PARAMETER_PREFIX}{sequence}");
                sequence += 1;
                while parameters.contains_key(&parameter_name) {
                    parameter_name = format!("{SET_PARAMETER_PREFIX}{sequence}");
                    sequence += 1;
                }

                parameters.insert(
                    parameter_name.clone(),
                    serialize_jsonb_write_value(Some(column), value),
                );
                clauses.push((column_name.clone(), format!(":{parameter_name}")));
            }

            let caller_set_updated_at = clauses
                .iter()
                .any(|(column_name, _)| column_name == UPDATED_AT_COLUMN_NAME);

            Ok(if caller_set_updated_at {
                clauses
            } else {
                with_updated_at_maintenance(state.shape, clauses)
            })
        }
    }
}

pub fn build_mutation_statement(
    state: &MutationStatementState<'_>,
) -> Result<MutationStatement, RepositoryError> {
    let aliased_table = format!(
        "{}.{} AS {}",
        quote_identifier(&state.shape.schema_name)?,
        quote_identifier(&state.shape.table_name)?,
        quote_identifier(state.alias)?
    );

    let mut parameters = state.where_parameters.clone();
    let set_clauses = build_set_clauses(state, &mut parameters)?;

    let filter_expression =
        render_user_where_expression(&state.where_clauses, &state.row_access_conditions);

    let should_skip_trashed_rows = state.kind == MutationKind::SoftDelete && !state.include_deleted;

    let where_expression = if should_skip_trashed_rows {
        let live_row_predicate = format!(
            "{}.{} IS NULL",
            quote_identifier(state.alias)?,
            quote_identifier(DELETED_AT_COLUMN_NAME)?
        );

        if filter_expression.is_empty() {
            live_row_predicate
        } else {
            format!("({filter_expression}) AND {live_row_predicate}")
        }
    } else {
        filter_expression
    };

    let mut returning: Vec<String> = Vec::with_capacity(state.returning_columns.len());
    for column_name in &state.returning_columns {
        returning.push(format!(
            "{}.{} AS {}",
            quote_identifier(state.alias)?,
            quote_identifier(column_name)?,
            quote_identifier(&build_column_result_alias(state.alias, column_name))?
        ));
    }

    let mut parts: Vec<String> = Vec::with_capacity(4);

    if state.kind == MutationKind::Delete {
        parts.push(format!("DELETE FROM {aliased_table}"));
    } else {
        if set_clauses.is_empty() {
            return Err(RepositoryError::invalid_query(format!(
                "An UPDATE on \"{}\" needs at least one column to set",
                state.shape.name_singular
            )));
        }

        let set: Vec<String> = clauses_to_set(&set_clauses)?;
        parts.push(format!("UPDATE {aliased_table}"));
        parts.push(format!("SET {}", set.join(", ")));
    }

    if !where_expression.is_empty() {
        parts.push(format!("WHERE {where_expression}"));
    }

    if !returning.is_empty() {
        parts.push(format!("RETURNING {}", returning.join(", ")));
    }

    Ok(MutationStatement {
        sql: parts.join(" "),
        parameters,
    })
}

fn clauses_to_set(clauses: &[(String, String)]) -> Result<Vec<String>, RepositoryError> {
    let mut set: Vec<String> = Vec::with_capacity(clauses.len());

    for (column_name, value_expression) in clauses {
        set.push(format!(
            "{} = {value_expression}",
            quote_identifier(column_name)?
        ));
    }

    Ok(set)
}

pub fn check_update_many_limit(count: usize) -> Result<(), RepositoryError> {
    if count > QUERY_MAX_RECORDS {
        return Err(RepositoryError::too_many_records_to_update());
    }

    Ok(())
}

pub fn build_update_statement(
    shape: &TableShape,
    alias: &str,
    data: &Map<String, Value>,
    where_clauses: Vec<MutationWhereClause>,
    where_parameters: Map<String, Value>,
    returning_columns: Vec<String>,
) -> Result<MutationStatement, RepositoryError> {
    let set_columns = expand_record_columns(shape, data)?;
    let state = MutationStatementState {
        alias,
        shape,
        kind: MutationKind::Update,
        set_columns,
        where_clauses,
        where_parameters,
        row_access_conditions: Vec::new(),
        include_deleted: false,
        returning_columns,
    };

    build_mutation_statement(&state)
}

pub fn build_soft_delete_statement(
    shape: &TableShape,
    alias: &str,
    where_clauses: Vec<MutationWhereClause>,
    where_parameters: Map<String, Value>,
    returning_columns: Vec<String>,
) -> Result<MutationStatement, RepositoryError> {
    let state = MutationStatementState {
        alias,
        shape,
        kind: MutationKind::SoftDelete,
        set_columns: Vec::new(),
        where_clauses,
        where_parameters,
        row_access_conditions: Vec::new(),
        include_deleted: false,
        returning_columns,
    };

    build_mutation_statement(&state)
}

pub fn build_restore_statement(
    shape: &TableShape,
    alias: &str,
    where_clauses: Vec<MutationWhereClause>,
    where_parameters: Map<String, Value>,
    returning_columns: Vec<String>,
) -> Result<MutationStatement, RepositoryError> {
    let state = MutationStatementState {
        alias,
        shape,
        kind: MutationKind::Restore,
        set_columns: Vec::new(),
        where_clauses,
        where_parameters,
        row_access_conditions: Vec::new(),
        include_deleted: false,
        returning_columns,
    };

    build_mutation_statement(&state)
}

pub fn build_delete_statement(
    shape: &TableShape,
    alias: &str,
    where_clauses: Vec<MutationWhereClause>,
    where_parameters: Map<String, Value>,
    returning_columns: Vec<String>,
) -> Result<MutationStatement, RepositoryError> {
    let state = MutationStatementState {
        alias,
        shape,
        kind: MutationKind::Delete,
        set_columns: Vec::new(),
        where_clauses,
        where_parameters,
        row_access_conditions: Vec::new(),
        include_deleted: false,
        returning_columns,
    };

    build_mutation_statement(&state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::flat::FieldMetadataType;
    use crate::metadata::flat::FlatEntityMaps;
    use crate::metadata::flat::FlatFieldMetadata;
    use crate::metadata::flat::FlatObjectMetadata;
    use crate::query::repository::table_shape::build_workspace_table_shape;
    use crate::query::runner::test_support::record;
    use uuid::Uuid;

    fn shape_from_fields(fields: Vec<(&str, FieldMetadataType, Option<Value>)>) -> TableShape {
        let mut maps: FlatEntityMaps<FlatFieldMetadata> = FlatEntityMaps::default();
        let mut field_ids: Vec<String> = Vec::new();

        for (index, (name, field_type, settings)) in fields.into_iter().enumerate() {
            let id = Uuid::from_u128(index as u128 + 1);
            let field = FlatFieldMetadata {
                id,
                universal_identifier: id.to_string(),
                object_metadata_id: Uuid::nil(),
                object_metadata_universal_identifier: "object-id".to_string(),
                field_type,
                name: name.to_string(),
                label: name.to_string(),
                settings,
                ..Default::default()
            };

            field_ids.push(id.to_string());

            let universal_identifier = field.universal_identifier.clone();
            maps.add(
                id.to_string(),
                field,
                universal_identifier,
                "application-id",
            );
        }

        let object = FlatObjectMetadata {
            id: Uuid::nil(),
            universal_identifier: "object-id".to_string(),
            name_singular: "person".to_string(),
            name_plural: "people".to_string(),
            label_singular: "Person".to_string(),
            label_plural: "People".to_string(),
            application_universal_identifier:
                crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER.to_string(),
            target_table_name: Some("person".to_string()),
            field_ids,
            ..Default::default()
        };

        build_workspace_table_shape(&Uuid::nil(), &object, &maps)
    }

    fn capture_shape() -> TableShape {
        shape_from_fields(vec![
            ("id", FieldMetadataType::Uuid, None),
            ("nameFirstName", FieldMetadataType::Text, None),
            ("jobTitle", FieldMetadataType::Text, None),
        ])
    }

    fn mutation_shape() -> TableShape {
        shape_from_fields(vec![
            ("id", FieldMetadataType::Uuid, None),
            ("jobTitle", FieldMetadataType::Text, None),
            ("deletedAt", FieldMetadataType::DateTime, None),
            ("updatedAt", FieldMetadataType::DateTime, None),
        ])
    }

    #[test]
    fn the_person_insert_matches_the_captured_upstream_statement() {
        let shape = capture_shape();
        let sql = build_insert_statement(
            &shape,
            &["id".to_string(), "jobTitle".to_string()],
            &[vec![
                InsertRowValue::Parameter("ormInsert_0".to_string()),
                InsertRowValue::Parameter("ormInsert_1".to_string()),
            ]],
            &["id".to_string(), "jobTitle".to_string()],
            false,
        )
        .expect("insert statement");

        assert_eq!(
            sql,
            "INSERT INTO \"workspace_0\".\"person\" (\"id\", \"jobTitle\") VALUES (:ormInsert_0, :ormInsert_1) RETURNING \"id\", \"jobTitle\""
        );
    }

    #[test]
    fn the_company_soft_delete_matches_the_captured_upstream_statement() {
        let shape = mutation_shape();
        let statement = build_soft_delete_statement(
            &shape,
            "person",
            vec![MutationWhereClause::and("(\"person\".\"id\" IN (:...ids))")],
            Map::new(),
            vec!["id".to_string(), "nameFirstName".to_string()],
        )
        .expect("soft delete statement");

        assert_eq!(
            statement.sql,
            "UPDATE \"workspace_0\".\"person\" AS \"person\" SET \"deletedAt\" = CURRENT_TIMESTAMP, \"updatedAt\" = CURRENT_TIMESTAMP WHERE ((\"person\".\"id\" IN (:...ids))) AND \"person\".\"deletedAt\" IS NULL RETURNING \"person\".\"id\" AS \"person_id\", \"person\".\"nameFirstName\" AS \"person_nameFirstName\""
        );
    }

    #[test]
    fn the_upstream_mutation_spec_captured_statement_reproduces() {
        let shape = mutation_shape();
        let mut where_parameters = Map::new();
        where_parameters.insert(
            "jobTitle".to_string(),
            Value::String("Tech Lead".to_string()),
        );

        let statement = build_update_statement(
            &shape,
            "person",
            &serde_json::json!({ "jobTitle": "Tech Lead" })
                .as_object()
                .unwrap()
                .clone(),
            vec![MutationWhereClause::and("\"person\".\"id\" = :id")],
            where_parameters,
            Vec::new(),
        )
        .expect("update statement");

        assert_eq!(
            statement.sql,
            "UPDATE \"workspace_0\".\"person\" AS \"person\" SET \"jobTitle\" = :ormSet_0, \"updatedAt\" = CURRENT_TIMESTAMP WHERE \"person\".\"id\" = :id"
        );
    }

    #[test]
    fn build_insert_rows_defaults_columns_a_row_omits() {
        let shape = capture_shape();
        let record: Map<String, Value> = serde_json::from_str(
            "{\"id\": \"20202020-0000-4000-8000-000000000001\", \"jobTitle\": \"CEO\"}",
        )
        .expect("record json");
        let other: Map<String, Value> =
            serde_json::from_str("{\"id\": \"20202020-0000-4000-8000-000000000002\"}")
                .expect("record json");

        let insert = build_insert_rows(&shape, &[record, other]).expect("insert rows");

        assert_eq!(
            insert.column_names,
            vec!["id".to_string(), "jobTitle".to_string()]
        );
        assert_eq!(
            insert.rows[1],
            vec![
                InsertRowValue::Parameter("ormInsert_2".to_string()),
                InsertRowValue::Default,
            ]
        );
        assert_eq!(
            insert.parameters["ormInsert_1"],
            Value::String("CEO".to_string())
        );

        let sql = build_insert_statement(
            &shape,
            &insert.column_names,
            &insert.rows,
            &["id".to_string()],
            false,
        )
        .expect("insert statement");

        assert_eq!(
            sql,
            "INSERT INTO \"workspace_0\".\"person\" (\"id\", \"jobTitle\") VALUES (:ormInsert_0, :ormInsert_1), (:ormInsert_2, DEFAULT) RETURNING \"id\""
        );
    }

    #[test]
    fn composite_input_expands_to_sub_columns_in_row_order() {
        let shape = shape_from_fields(vec![
            ("id", FieldMetadataType::Uuid, None),
            ("name", FieldMetadataType::FullName, None),
        ]);
        let record: Map<String, Value> = serde_json::from_str(
            "{\"name\": {\"firstName\": \"Ada\", \"lastName\": \"Lovelace\"}}",
        )
        .expect("record json");

        let expanded = expand_record_columns(&shape, &record).expect("expand");

        assert_eq!(
            expanded,
            vec![
                (
                    "nameFirstName".to_string(),
                    Value::String("Ada".to_string())
                ),
                (
                    "nameLastName".to_string(),
                    Value::String("Lovelace".to_string())
                ),
            ]
        );
    }

    #[test]
    fn an_actor_composite_expands_all_four_sub_columns() {
        let shape = shape_from_fields(vec![
            ("id", FieldMetadataType::Uuid, None),
            ("createdBy", FieldMetadataType::Actor, None),
        ]);
        let record: Map<String, Value> = serde_json::from_str(
            "{\"createdBy\": {\"source\": \"MANUAL\", \"workspaceMemberId\": \"20202020-0000-4000-8000-000000000009\", \"name\": \"Ada\", \"context\": {}}}",
        )
        .expect("record json");

        let expanded = expand_record_columns(&shape, &record).expect("expand");

        let names: Vec<&str> = expanded.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "createdBySource",
                "createdByWorkspaceMemberId",
                "createdByName",
                "createdByContext",
            ]
        );
    }

    #[test]
    fn an_unknown_column_is_rejected_by_expansion() {
        let shape = capture_shape();
        let record: Map<String, Value> =
            serde_json::from_str("{\"nope\": 1}").expect("record json");

        let error = expand_record_columns(&shape, &record).expect_err("unknown column");
        assert_eq!(error.code(), "UNKNOWN_COLUMN");
        assert!(error.message().contains("nope"));
    }

    #[test]
    fn soft_delete_on_a_deleted_at_less_object_is_unsupported() {
        let shape = shape_from_fields(vec![("id", FieldMetadataType::Uuid, None)]);
        let error =
            build_soft_delete_statement(&shape, "person", Vec::new(), Map::new(), Vec::new())
                .expect_err("unsupported");
        assert_eq!(error.code(), "UNSUPPORTED_OPERATION");

        let error = build_restore_statement(&shape, "person", Vec::new(), Map::new(), Vec::new())
            .expect_err("unsupported");
        assert_eq!(error.code(), "UNSUPPORTED_OPERATION");
    }

    #[test]
    fn restore_clears_deleted_at_and_stamps_updated_at() {
        let shape = mutation_shape();
        let statement = build_restore_statement(
            &shape,
            "person",
            vec![MutationWhereClause::and("(\"person\".\"id\" IN (:...ids))")],
            Map::new(),
            vec!["id".to_string()],
        )
        .expect("restore statement");

        assert_eq!(
            statement.sql,
            "UPDATE \"workspace_0\".\"person\" AS \"person\" SET \"deletedAt\" = NULL, \"updatedAt\" = CURRENT_TIMESTAMP WHERE (\"person\".\"id\" IN (:...ids)) RETURNING \"person\".\"id\" AS \"person_id\""
        );
    }

    #[test]
    fn hard_delete_emits_no_set_clause() {
        let shape = mutation_shape();
        let statement = build_delete_statement(
            &shape,
            "person",
            vec![
                MutationWhereClause::and("(\"person\".\"id\" = :a)"),
                MutationWhereClause::or("(\"person\".\"id\" = :b)"),
            ],
            Map::new(),
            vec![],
        )
        .expect("delete statement");

        assert_eq!(
            statement.sql,
            "DELETE FROM \"workspace_0\".\"person\" AS \"person\" WHERE (\"person\".\"id\" = :a) OR (\"person\".\"id\" = :b)"
        );
    }

    #[test]
    fn row_access_conditions_stay_outside_the_user_expression() {
        let shape = mutation_shape();
        let statement = build_delete_statement(
            &shape,
            "person",
            vec![
                MutationWhereClause::and("(\"person\".\"id\" = :a)"),
                MutationWhereClause::or("(\"person\".\"id\" = :b)"),
            ],
            Map::new(),
            vec![],
        )
        .expect("delete without access");

        assert_eq!(
            statement.sql,
            "DELETE FROM \"workspace_0\".\"person\" AS \"person\" WHERE (\"person\".\"id\" = :a) OR (\"person\".\"id\" = :b)"
        );

        let state = MutationStatementState {
            alias: "person",
            shape: &shape,
            kind: MutationKind::Delete,
            set_columns: Vec::new(),
            where_clauses: vec![
                MutationWhereClause::and("(\"person\".\"id\" = :a)"),
                MutationWhereClause::or("(\"person\".\"id\" = :b)"),
            ],
            where_parameters: Map::new(),
            row_access_conditions: vec!["\"person\".\"ownerId\" = :ownerId".to_string()],
            include_deleted: false,
            returning_columns: Vec::new(),
        };

        let statement = build_mutation_statement(&state).expect("delete with access");
        assert_eq!(
            statement.sql,
            "DELETE FROM \"workspace_0\".\"person\" AS \"person\" WHERE ((\"person\".\"id\" = :a) OR (\"person\".\"id\" = :b)) AND (\"person\".\"ownerId\" = :ownerId)"
        );
    }

    #[test]
    fn an_update_without_columns_still_stamps_updated_at() {
        let shape = mutation_shape();
        let statement = build_update_statement(
            &shape,
            "person",
            &Map::new(),
            vec![MutationWhereClause::and("\"person\".\"id\" = :id")],
            Map::new(),
            Vec::new(),
        )
        .expect("update statement");

        assert_eq!(
            statement.sql,
            "UPDATE \"workspace_0\".\"person\" AS \"person\" SET \"updatedAt\" = CURRENT_TIMESTAMP WHERE \"person\".\"id\" = :id"
        );
    }

    #[test]
    fn a_caller_set_updated_at_skips_maintenance() {
        let shape = mutation_shape();
        let mut parameters = Map::new();
        parameters.insert(
            "jobTitle".to_string(),
            Value::String("Tech Lead".to_string()),
        );

        let mut data = Map::new();
        data.insert(
            "jobTitle".to_string(),
            Value::String("Tech Lead".to_string()),
        );
        data.insert("updatedAt".to_string(), Value::Null);

        let statement = build_update_statement(
            &shape,
            "person",
            &data,
            vec![MutationWhereClause::and("\"person\".\"id\" = :id")],
            parameters,
            Vec::new(),
        )
        .expect("update statement");

        assert_eq!(
            statement.sql,
            "UPDATE \"workspace_0\".\"person\" AS \"person\" SET \"jobTitle\" = :ormSet_0, \"updatedAt\" = :ormSet_1 WHERE \"person\".\"id\" = :id"
        );
        assert_eq!(statement.parameters["ormSet_1"], Value::Null);
    }

    #[test]
    fn a_table_without_updated_at_keeps_the_plain_set() {
        let shape = shape_from_fields(vec![
            ("id", FieldMetadataType::Uuid, None),
            ("jobTitle", FieldMetadataType::Text, None),
        ]);
        let statement = build_update_statement(
            &shape,
            "person",
            &serde_json::json!({"jobTitle": "CEO"})
                .as_object()
                .unwrap()
                .clone(),
            vec![MutationWhereClause::and("\"person\".\"id\" = :id")],
            Map::new(),
            Vec::new(),
        )
        .expect("update statement");

        assert_eq!(
            statement.sql,
            "UPDATE \"workspace_0\".\"person\" AS \"person\" SET \"jobTitle\" = :ormSet_0 WHERE \"person\".\"id\" = :id"
        );
    }

    #[test]
    fn an_insert_without_columns_or_rows_is_invalid() {
        let shape = capture_shape();
        let error =
            build_insert_statement(&shape, &[], &[], &[], false).expect_err("invalid insert");
        assert_eq!(error.code(), "INVALID_QUERY");
    }

    #[test]
    fn update_many_above_the_limit_is_too_many_records() {
        assert!(check_update_many_limit(QUERY_MAX_RECORDS).is_ok());
        let error = check_update_many_limit(QUERY_MAX_RECORDS + 1).expect_err("too many");
        assert_eq!(error.code(), "TOO_MANY_RECORDS_TO_UPDATE");
        assert!(error.message().contains("200"));
    }

    #[test]
    fn jsonb_columns_bind_as_json_text() {
        let shape = shape_from_fields(vec![
            ("id", FieldMetadataType::Uuid, None),
            ("overrides", FieldMetadataType::RawJson, None),
        ]);
        let record: Map<String, Value> =
            serde_json::from_str("{\"overrides\": {\"a\": [1, 2]}}").expect("record json");

        let insert = build_insert_rows(&shape, &[record]).expect("insert rows");

        assert_eq!(
            insert.parameters["ormInsert_0"],
            Value::String("{\"a\":[1,2]}".to_string())
        );
    }

    #[test]
    fn the_named_parameters_compile_for_execution() {
        let shape = mutation_shape();
        let mut where_parameters = Map::new();
        where_parameters.insert("id".to_string(), Value::String("id-1".to_string()));

        let statement = build_soft_delete_statement(
            &shape,
            "person",
            vec![MutationWhereClause::and("\"person\".\"id\" = :id")],
            where_parameters,
            vec!["id".to_string()],
        )
        .expect("soft delete statement");

        let compiled = statement.compile().expect("compiled statement");

        assert_eq!(
            compiled.text,
            "UPDATE \"workspace_0\".\"person\" AS \"person\" SET \"deletedAt\" = CURRENT_TIMESTAMP, \"updatedAt\" = CURRENT_TIMESTAMP WHERE (\"person\".\"id\" = $1) AND \"person\".\"deletedAt\" IS NULL RETURNING \"person\".\"id\" AS \"person_id\""
        );
        assert_eq!(compiled.values, vec![Value::String("id-1".to_string())]);
    }

    #[test]
    fn a_long_result_alias_is_hashed_to_the_postgres_limit() {
        let alias = "a".repeat(40);
        let column = "b".repeat(40);
        let result = build_column_result_alias(&alias, &column);

        assert_eq!(result.len(), POSTGRES_MAX_ALIAS_LENGTH);
        assert!(result
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }

    fn actor_heavy_shape() -> TableShape {
        shape_from_fields(vec![
            ("id", FieldMetadataType::Uuid, None),
            ("name", FieldMetadataType::Text, None),
            ("position", FieldMetadataType::Number, None),
            ("companyId", FieldMetadataType::Uuid, None),
            ("createdBy", FieldMetadataType::Actor, None),
            ("updatedBy", FieldMetadataType::Actor, None),
        ])
    }

    fn actor_record(name: &str) -> Map<String, Value> {
        record(&format!(
            "{{\"name\": \"{name}\", \"createdBy\": {{\"source\": \"MANUAL\", \
                 \"workspaceMemberId\": \"20202020-0000-4000-8000-000000000009\", \
                 \"name\": \"Ada\", \"context\": {{}}}}, \
                 \"updatedBy\": {{\"source\": \"MANUAL\", \
                 \"workspaceMemberId\": \"20202020-0000-4000-8000-000000000009\", \
                 \"name\": \"Ada\", \"context\": {{}}}}, \"position\": 1}}"
        ))
    }

    #[test]
    fn a_single_row_insert_casts_the_actor_columns_without_rewriting_other_placeholders() {
        let shape = actor_heavy_shape();
        let rows = build_insert_rows(&shape, &[actor_record("Bulk One")]).expect("one row");
        let execution =
            build_insert_execution(&shape, &rows, &["id".to_string()], false).expect("execution");

        assert_eq!(
            execution.columns,
            vec![
                "name".to_string(),
                "createdBySource".to_string(),
                "createdByWorkspaceMemberId".to_string(),
                "createdByName".to_string(),
                "createdByContext".to_string(),
                "updatedBySource".to_string(),
                "updatedByWorkspaceMemberId".to_string(),
                "updatedByName".to_string(),
                "updatedByContext".to_string(),
                "position".to_string(),
            ]
        );

        let compiled =
            compile_named_parameters(&execution.sql, &rows.parameters).expect("compiled");

        assert_eq!(
            compiled.text,
            "INSERT INTO \"workspace_0\".\"person\" (\"name\", \"createdBySource\", \
             \"createdByWorkspaceMemberId\", \"createdByName\", \"createdByContext\", \
             \"updatedBySource\", \"updatedByWorkspaceMemberId\", \"updatedByName\", \
             \"updatedByContext\", \"position\") VALUES ($1, $2::text::\"workspace_0\".\"\
             person_createdBySource_enum\", $3::uuid, $4, $5::jsonb, $6::text::\"workspace_0\".\"\
             person_updatedBySource_enum\", $7::uuid, $8, $9::jsonb, $10) RETURNING \"id\""
        );
        assert_eq!(compiled.values.len(), 10);
    }

    #[test]
    fn an_insert_casts_relation_json_and_timestamp_columns_by_metadata_type() {
        let shape = shape_from_fields(vec![
            ("id", FieldMetadataType::Uuid, None),
            ("name", FieldMetadataType::Text, None),
            ("companyId", FieldMetadataType::Uuid, None),
            ("labels", FieldMetadataType::RawJson, None),
            ("createdAt", FieldMetadataType::DateTime, None),
            ("birthday", FieldMetadataType::Date, None),
        ]);
        let rows = build_insert_rows(
            &shape,
            &[record(
                "{\"name\": \"Rel\", \"companyId\": null, \"labels\": {\"a\": 1}, \
                 \"createdAt\": \"2026-01-02T03:04:05.000Z\", \"birthday\": \"2026-01-02\"}",
            )],
        )
        .expect("one row");
        let execution =
            build_insert_execution(&shape, &rows, &["id".to_string()], false).expect("execution");
        let compiled =
            compile_named_parameters(&execution.sql, &rows.parameters).expect("compiled");

        assert_eq!(
            compiled.text,
            "INSERT INTO \"workspace_0\".\"person\" (\"name\", \"companyId\", \"labels\", \
             \"createdAt\", \"birthday\") VALUES ($1, $2::uuid, $3::jsonb, $4::timestamptz, \
             $5::date) RETURNING \"id\""
        );
        assert_eq!(compiled.values.len(), 5);
    }

    #[test]
    fn a_two_row_insert_casts_each_actor_column_and_never_eats_a_placeholder_digit() {
        let shape = actor_heavy_shape();
        let rows = build_insert_rows(
            &shape,
            &[actor_record("Bulk One"), actor_record("Bulk Two")],
        )
        .expect("two rows");
        let execution =
            build_insert_execution(&shape, &rows, &["id".to_string()], false).expect("execution");

        assert_eq!(execution.columns.len(), 20);
        assert_eq!(execution.columns[9], "position".to_string());
        assert_eq!(execution.columns[10], "name".to_string());
        assert_eq!(execution.columns[19], "position".to_string());

        let compiled =
            compile_named_parameters(&execution.sql, &rows.parameters).expect("compiled");
        let tail = compiled
            .text
            .split("), (")
            .last()
            .expect("second value tuple")
            .to_string();

        assert_eq!(
            tail,
            "$11, $12::text::\"workspace_0\".\"person_createdBySource_enum\", $13::uuid, $14, \
             $15::jsonb, $16::text::\"workspace_0\".\"person_updatedBySource_enum\", $17::uuid, \
             $18, $19::jsonb, $20) RETURNING \"id\""
        );
        assert!(
            !compiled.text.contains("_enum0"),
            "a cast must not be spliced into a longer placeholder: {}",
            compiled.text
        );
        assert_eq!(compiled.values.len(), 20);
    }

    #[test]
    fn a_defaulted_cell_keeps_every_later_bind_on_its_own_column() {
        let shape = actor_heavy_shape();
        let rows = build_insert_rows(
            &shape,
            &[
                record(
                    "{\"name\": \"With Company\", \"companyId\": \
                     \"30303030-0000-4000-8000-000000000004\", \
                     \"createdBy\": {\"source\": \"MANUAL\", \
                     \"workspaceMemberId\": \"20202020-0000-4000-8000-000000000009\", \
                     \"name\": \"Ada\", \"context\": {}}, \
                     \"updatedBy\": {\"source\": \"MANUAL\", \
                     \"workspaceMemberId\": \"20202020-0000-4000-8000-000000000009\", \
                     \"name\": \"Ada\", \"context\": {}}, \"position\": 1}",
                ),
                record(
                    "{\"name\": \"No Company\", \"createdBy\": {\"source\": \"MANUAL\", \
                     \"workspaceMemberId\": null, \"name\": null, \"context\": {}}, \
                     \"updatedBy\": {\"source\": \"MANUAL\", \"workspaceMemberId\": null, \
                     \"name\": null, \"context\": {}}, \"position\": 2}",
                ),
            ],
        )
        .expect("heterogeneous rows");
        let execution =
            build_insert_execution(&shape, &rows, &["id".to_string()], false).expect("execution");

        assert_eq!(rows.column_names.len(), 11);
        assert_eq!(
            execution.columns,
            vec![
                "name".to_string(),
                "companyId".to_string(),
                "createdBySource".to_string(),
                "createdByWorkspaceMemberId".to_string(),
                "createdByName".to_string(),
                "createdByContext".to_string(),
                "updatedBySource".to_string(),
                "updatedByWorkspaceMemberId".to_string(),
                "updatedByName".to_string(),
                "updatedByContext".to_string(),
                "position".to_string(),
                "name".to_string(),
                "createdBySource".to_string(),
                "createdByWorkspaceMemberId".to_string(),
                "createdByName".to_string(),
                "createdByContext".to_string(),
                "updatedBySource".to_string(),
                "updatedByWorkspaceMemberId".to_string(),
                "updatedByName".to_string(),
                "updatedByContext".to_string(),
                "position".to_string(),
            ]
        );

        let compiled =
            compile_named_parameters(&execution.sql, &rows.parameters).expect("compiled");

        assert_eq!(compiled.values.len(), execution.columns.len());
        assert!(execution.sql.contains("DEFAULT"));
        assert!(
            compiled
                .text
                .contains("$3::text::\"workspace_0\".\"person_createdBySource_enum\", $4::uuid"),
            "{}",
            compiled.text
        );
        assert!(compiled.text.contains("$21"), "{}", compiled.text);
        assert!(!compiled.text.contains("$21:"), "{}", compiled.text);
    }

    #[test]
    fn set_casts_survive_more_than_ten_set_columns() {
        let shape = actor_heavy_shape();
        let mut sql = String::from("UPDATE \"person\" SET ");
        let mut set_columns: Vec<String> = Vec::new();

        for (index, column_name) in shape
            .column_names
            .iter()
            .filter(|name| name.as_str() != "position")
            .enumerate()
        {
            if index > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&format!("\"{column_name}\" = :ormSet_{index}"));
            set_columns.push(column_name.clone());
        }
        assert!(
            set_columns.len() > 10,
            "the fixture must exercise the two-digit placeholder range"
        );

        let casted = cast_set_parameters(&sql, &shape, &set_columns).expect("cast set parameters");
        let mut expected = String::from("UPDATE \"person\" SET ");

        for (index, column_name) in set_columns.iter().enumerate() {
            if index > 0 {
                expected.push_str(", ");
            }
            expected.push_str(&format!(
                "\"{column_name}\" = :ormSet_{index}{}",
                set_column_cast(&shape, column_name).expect("cast")
            ));
        }

        assert_eq!(casted, expected);
        assert!(!casted.contains("_enum0"));
        assert!(casted.contains(":ormSet_10"));
    }

    #[test]
    fn cast_named_parameters_leaves_unknown_tokens_alone() {
        let sql = "SELECT * FROM \"t\" WHERE \"id\" = :id AND \"a\" = :ormInsert_0";
        let shape = actor_heavy_shape();
        let (rewritten, matched) = cast_named_parameters(sql, &|name| {
            if name == "ormInsert_0" {
                return Ok(Some(String::from("::uuid")));
            }

            Ok(None)
        })
        .expect("cast");

        assert_eq!(
            rewritten,
            "SELECT * FROM \"t\" WHERE \"id\" = :id AND \"a\" = :ormInsert_0::uuid"
        );
        assert_eq!(matched, vec!["ormInsert_0".to_string()]);
        assert!(shape.column("companyId").is_some());
    }

    #[test]
    fn an_unterminated_quoted_token_is_rejected_before_casting() {
        let error = cast_named_parameters("\"person.id = :ormInsert_0", &|_| {
            Ok(Some(String::from("::uuid")))
        })
        .expect_err("unterminated");

        assert_eq!(error.code(), "INVALID_QUERY");
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::metadata::flat::load_workspace_flat_maps;
    use crate::query::repository::table_shape::TableShapeCache;
    use crate::state::AppState;
    use crate::workspace_migrations::runner::bump_metadata_version;
    use nudge_core::NodeEnv;
    use std::collections::HashSet;
    use std::sync::Arc;
    use uuid::Uuid;

    static DB_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn test_config(pg_database_url: String) -> nudge_core::Config {
        nudge_core::Config {
            node_env: NodeEnv::Test,
            node_port: 3998,
            server_url: "http://localhost:3998".to_string(),
            pg_database_url,
            pg_pool_max_connections: 4,
            redis_url: "redis://127.0.0.1:6399".to_string(),
            redis_cache_prefix: String::new(),
            app_secret: "repository-db-test-secret".to_string(),
            encryption_key: None,
            fallback_encryption_key: None,
            access_token_expires_in_secs: 1800,
            refresh_token_expires_in_secs: 3600,
            login_token_expires_in_secs: 900,
            workspace_agnostic_token_expires_in_secs: 1800,
            refresh_token_reuse_grace_period_secs: 60,
            frontend_url: "http://localhost:3998".to_string(),
            auth_cookie_same_site: "lax".to_string(),
            auth_cookie_allowed_origins: String::new(),
            session_absolute_lifetime_secs: 3600,
            session_idle_timeout_secs: 3600,
            is_cloudflare_geolocation_enabled: false,
            workspace_schema_ddl_locked: false,
            search_ilike_fallback_timeout_ms: 2000,
            allow_requests_to_twenty_icons: true,
            file_token_expires_in_secs: 86400,
            storage_local_path: ".local-storage".into(),
        }
    }

    struct SeededWorkspace {
        state: AppState,
        workspace_id: Uuid,
        schema_name: String,
    }

    async fn seeded_workspace(url: &str) -> Option<SeededWorkspace> {
        let state = AppState::connect(&test_config(url.to_string()))
            .await
            .expect("connecting repository db test state");

        crate::workspace_migrations::ddl::test_support::migrate_core(url)
            .await
            .expect("running core migrations");

        let workspace_id = Uuid::new_v4();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let application_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let mut fixture = state.core_pool.begin().await.expect("fixture tx");

        sqlx::query(
            "INSERT INTO core.\"workspace\" \
             (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
              \"workspaceCustomApplicationId\", \"isPasswordAuthEnabled\", \"databaseSchema\") \
             VALUES ($1, 'Repository Parity', $2, 'PENDING_CREATION', $3, $4, true, $5)",
        )
        .bind(workspace_id)
        .bind(Uuid::new_v4())
        .bind(format!("repo-{}", &workspace_id.simple().to_string()[..8]))
        .bind(application_id)
        .bind(&schema_name)
        .execute(&mut *fixture)
        .await
        .expect("inserting the repository workspace");

        sqlx::query(
            "INSERT INTO core.\"application\" \
             (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", \"workspaceId\") \
             VALUES ($1, $2, 'Repository Application', 'LOCAL', 'INSTALLED', '', $3)",
        )
        .bind(application_id)
        .bind(Uuid::new_v4())
        .bind(workspace_id)
        .execute(&mut *fixture)
        .await
        .expect("inserting the repository application");

        sqlx::query(
            "INSERT INTO core.\"role\" \
             (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\") \
             VALUES ($1, $2, 'Member', $3, $4)",
        )
        .bind(role_id)
        .bind(Uuid::new_v4())
        .bind(application_id)
        .bind(workspace_id)
        .execute(&mut *fixture)
        .await
        .expect("inserting the default role");

        sqlx::query(
            "UPDATE core.\"workspace\" SET \"activationStatus\" = 'CREATED', \
                \"defaultRoleId\" = $2 WHERE id = $1",
        )
        .bind(workspace_id)
        .bind(role_id)
        .execute(&mut *fixture)
        .await
        .expect("activating the repository workspace");

        fixture.commit().await.expect("committing the fixture");

        sqlx::query(&format!("CREATE SCHEMA \"{schema_name}\""))
            .execute(&state.core_pool)
            .await
            .expect("creating the repository schema");

        let outcome = crate::standard_objects::seed_standard_objects(&state, workspace_id)
            .await
            .expect("seeding the standard objects");

        assert!(outcome.seeded);

        Some(SeededWorkspace {
            state,
            workspace_id,
            schema_name,
        })
    }

    async fn metadata_version(state: &AppState, workspace_id: Uuid) -> i32 {
        let row: (i32,) =
            sqlx::query_as("SELECT \"metadataVersion\" FROM core.\"workspace\" WHERE id = $1")
                .bind(workspace_id)
                .fetch_one(&state.core_pool)
                .await
                .expect("reading the metadata version");

        row.0
    }

    fn record(json: &str) -> Map<String, Value> {
        serde_json::from_str::<Value>(json)
            .expect("record json")
            .as_object()
            .expect("record object")
            .clone()
    }

    async fn physical_columns(state: &AppState, schema: &str, table: &str) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT a.attname::text FROM pg_attribute a \
             JOIN pg_class c ON c.oid = a.attrelid \
             JOIN pg_namespace n ON n.oid = c.relnamespace \
             WHERE n.nspname = $1 AND c.relname = $2 AND a.attnum > 0 AND NOT a.attisdropped \
             ORDER BY a.attnum",
        )
        .bind(schema)
        .bind(table)
        .fetch_all(&state.core_pool)
        .await
        .expect("reading physical columns")
    }

    async fn exec_insert_text(
        state: &AppState,
        sql: &str,
        parameters: &Map<String, Value>,
        text_columns: &[&str],
    ) -> Uuid {
        let compiled = compile_named_parameters(sql, parameters).expect("compiled insert");
        assert_eq!(compiled.values.len(), text_columns.len());

        let mut query = sqlx::query_scalar::<_, Uuid>(&compiled.text);
        for value in &compiled.values {
            query = query.bind(value.as_str().expect("text insert value").to_string());
        }

        query
            .fetch_one(&state.core_pool)
            .await
            .expect("person insert executes")
    }

    async fn exec_mutation_with_id(
        state: &AppState,
        statement: &MutationStatement,
        id: Uuid,
    ) -> Vec<(Uuid, Option<String>)> {
        let compiled = statement.compile().expect("compiled statement");

        let query = sqlx::query_as::<_, (Uuid, Option<String>)>(&compiled.text).bind(id);

        query
            .fetch_all(&state.core_pool)
            .await
            .expect("mutation executes")
    }

    async fn exec_update_with_text_and_id(
        state: &AppState,
        statement: &MutationStatement,
        text: &str,
        id: Uuid,
    ) -> Vec<(Uuid, Option<String>)> {
        let compiled = statement.compile().expect("compiled statement");
        assert_eq!(compiled.values.len(), 2);

        let query = sqlx::query_as::<_, (Uuid, Option<String>)>(&compiled.text)
            .bind(text.to_string())
            .bind(id);

        query
            .fetch_all(&state.core_pool)
            .await
            .expect("update executes")
    }

    #[tokio::test]
    async fn the_emitted_person_and_company_statements_execute_and_resolve_every_column() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };

        let _guard = DB_TEST_LOCK.lock().await;

        let seeded = seeded_workspace(&url).await.expect("seeded workspace");
        let state = &seeded.state;
        let maps = load_workspace_flat_maps(&state.core_pool, seeded.workspace_id)
            .await
            .expect("loading the flat maps");
        let cache = TableShapeCache::new(16);
        let version = metadata_version(state, seeded.workspace_id).await;

        let find_object = |name: &str| {
            maps.object_metadata_maps
                .by_universal_identifier
                .values()
                .find(|object| object.name_singular == name)
                .expect("object exists")
                .clone()
        };
        let person_object = find_object("person");
        let company_object = find_object("company");

        let person = cache
            .get_or_build(
                &seeded.workspace_id,
                version,
                &person_object,
                &maps.field_metadata_maps,
            )
            .expect("the person shape");
        let company = cache
            .get_or_build(
                &seeded.workspace_id,
                version,
                &company_object,
                &maps.field_metadata_maps,
            )
            .expect("the company shape");

        assert_eq!(person.schema_name, seeded.schema_name);
        assert_eq!(person.table_name, "person");
        assert_eq!(company.table_name, "company");
        assert!(person.has_deleted_at_column);

        assert_eq!(
            person
                .relation("company")
                .and_then(|relation| relation.join_column_name.clone())
                .as_deref(),
            Some("companyId")
        );
        assert_eq!(
            person
                .relation("attachments")
                .expect("one to many relation")
                .join_column_name,
            None
        );
        assert_eq!(
            company
                .relation("accountOwner")
                .and_then(|relation| relation.join_column_name.clone())
                .as_deref(),
            Some("accountOwnerId")
        );

        let person_physical = physical_columns(state, &seeded.schema_name, "person").await;
        let company_physical = physical_columns(state, &seeded.schema_name, "company").await;

        let person_shape_set: HashSet<&str> =
            person.column_names.iter().map(String::as_str).collect();
        let company_shape_set: HashSet<&str> =
            company.column_names.iter().map(String::as_str).collect();
        let person_physical_set: HashSet<&str> = person_physical
            .iter()
            .map(String::as_str)
            .filter(|name| !name.starts_with("_accessGroup") && *name != "_isPrivate")
            .collect();
        let company_physical_set: HashSet<&str> = company_physical
            .iter()
            .map(String::as_str)
            .filter(|name| !name.starts_with("_accessGroup") && *name != "_isPrivate")
            .collect();

        assert_eq!(
            person_shape_set, person_physical_set,
            "person shape columns and physical columns must resolve identically"
        );
        assert_eq!(
            company_shape_set, company_physical_set,
            "company shape columns and physical columns must resolve identically"
        );
        assert!(person.column("nameFirstName").is_some());
        assert!(person.column("nameLastName").is_some());
        assert!(person.column("companyId").is_some());
        assert!(person.column("emailsPrimaryEmail").is_some());
        assert!(person.column("additionalEmails").is_none());
        assert!(person.column("emailsAdditionalEmails").is_some());
        assert!(person.column("createdByWorkspaceMemberId").is_some());
        assert!(person.column("searchVector").is_some());
        assert!(company.column("name").is_some());

        let company_insert = build_insert_rows(&company, &[record("{\"name\": \"Nudge Inc\"}")])
            .expect("company rows");
        let company_sql = build_insert_statement(
            &company,
            &company_insert.column_names,
            &company_insert.rows,
            &["id".to_string()],
            false,
        )
        .expect("company insert");
        assert!(company_sql.starts_with(&format!(
            "INSERT INTO \"{}\".\"company\" (\"name\") VALUES (:ormInsert_0) RETURNING \"id\"",
            seeded.schema_name
        )));

        let company_id =
            exec_insert_text(state, &company_sql, &company_insert.parameters, &["name"]).await;

        let first = record(
            "{\"name\": {\"firstName\": \"Ada\", \"lastName\": \"Lovelace\"}, \"jobTitle\": \"Engineer\"}",
        );
        let person_records = [
            first,
            record("{\"name\": {\"firstName\": \"Grace\", \"lastName\": \"Hopper\"}, \"jobTitle\": \"Rear Admiral\"}"),
        ];

        let person_insert = build_insert_rows(&person, &person_records).expect("person rows");
        assert_eq!(
            person_insert.column_names,
            vec![
                "nameFirstName".to_string(),
                "nameLastName".to_string(),
                "jobTitle".to_string(),
            ]
        );
        assert!(!person_insert.column_names.contains(&"name".to_string()));

        let person_sql = build_insert_statement(
            &person,
            &person_insert.column_names,
            &person_insert.rows,
            &["id".to_string()],
            false,
        )
        .expect("person insert");
        assert_eq!(
            person_sql,
            format!(
                "INSERT INTO \"{}\".\"person\" (\"nameFirstName\", \"nameLastName\", \"jobTitle\") \
                 VALUES (:ormInsert_0, :ormInsert_1, :ormInsert_2), (:ormInsert_3, :ormInsert_4, :ormInsert_5) \
                 RETURNING \"id\"",
                seeded.schema_name
            )
        );

        let person_rows = {
            let compiled = compile_named_parameters(&person_sql, &person_insert.parameters)
                .expect("compiled person insert");
            assert_eq!(compiled.values.len(), 6);
            let mut query = sqlx::query_scalar::<_, Uuid>(&compiled.text);
            for value in &compiled.values {
                query = query.bind(value.as_str().expect("text value").to_string());
            }
            query
                .fetch_all(&state.core_pool)
                .await
                .expect("person inserts")
        };
        assert_eq!(person_rows.len(), 2);
        let person_id = person_rows[0];

        let stored: (String, Option<String>) = sqlx::query_as(&format!(
            "SELECT \"nameFirstName\", \"jobTitle\" FROM \"{}\".\"person\" WHERE id = $1",
            seeded.schema_name
        ))
        .bind(person_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("reading the inserted person");

        assert_eq!(stored.0, "Ada");
        assert_eq!(stored.1.as_deref(), Some("Engineer"));

        let returned = vec!["id".to_string(), "jobTitle".to_string()];
        let mut where_parameters = Map::new();
        where_parameters.insert("id".to_string(), Value::String(person_id.to_string()));
        let where_id = vec![MutationWhereClause::and("\"person\".\"id\" = :id::uuid")];

        let soft = build_soft_delete_statement(
            &person,
            "person",
            where_id.clone(),
            where_parameters.clone(),
            returned.clone(),
        )
        .expect("soft delete statement");
        assert_eq!(
            soft.sql,
            format!(
                "UPDATE \"{}\".\"person\" AS \"person\" \
                 SET \"deletedAt\" = CURRENT_TIMESTAMP, \"updatedAt\" = CURRENT_TIMESTAMP \
                 WHERE (\"person\".\"id\" = :id::uuid) AND \"person\".\"deletedAt\" IS NULL \
                 RETURNING \"person\".\"id\" AS \"person_id\", \"person\".\"jobTitle\" AS \"person_jobTitle\"",
                seeded.schema_name
            )
        );
        assert_eq!(soft.parameters["id"], Value::String(person_id.to_string()));

        let deleted = exec_mutation_with_id(state, &soft, person_id).await;
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].0, person_id);
        assert_eq!(deleted[0].1.as_deref(), Some("Engineer"));

        let skipped = exec_mutation_with_id(state, &soft, person_id).await;
        assert!(
            skipped.is_empty(),
            "the deletedAt IS NULL predicate skips trashed rows"
        );

        let soft_compiled = soft.compile().expect("soft compiled");
        assert_eq!(
            soft_compiled.text,
            format!(
                "UPDATE \"{}\".\"person\" AS \"person\" \
                 SET \"deletedAt\" = CURRENT_TIMESTAMP, \"updatedAt\" = CURRENT_TIMESTAMP \
                 WHERE (\"person\".\"id\" = $1::uuid) AND \"person\".\"deletedAt\" IS NULL \
                 RETURNING \"person\".\"id\" AS \"person_id\", \"person\".\"jobTitle\" AS \"person_jobTitle\"",
                seeded.schema_name
            )
        );

        let restored = build_restore_statement(
            &person,
            "person",
            where_id.clone(),
            where_parameters.clone(),
            returned.clone(),
        )
        .expect("restore statement");
        assert_eq!(
            exec_mutation_with_id(state, &restored, person_id)
                .await
                .len(),
            1
        );

        let updated = build_update_statement(
            &person,
            "person",
            &record("{\"jobTitle\": \"Chief Engineer\"}"),
            where_id.clone(),
            where_parameters.clone(),
            returned.clone(),
        )
        .expect("update statement");
        let updated_rows =
            exec_update_with_text_and_id(state, &updated, "Chief Engineer", person_id).await;
        assert_eq!(updated_rows.len(), 1);
        assert_eq!(updated_rows[0].1.as_deref(), Some("Chief Engineer"));

        let bumped_updated_at: (bool,) = sqlx::query_as(&format!(
            "SELECT \"updatedAt\" > \"createdAt\" FROM \"{}\".\"person\" WHERE id = $1",
            seeded.schema_name
        ))
        .bind(person_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("reading updatedAt");
        assert!(
            bumped_updated_at.0,
            "updatedAt maintenance must stamp on every mutation"
        );

        let destroyed =
            build_delete_statement(&person, "person", where_id, where_parameters, returned)
                .expect("destroy statement");
        assert_eq!(
            exec_mutation_with_id(state, &destroyed, person_id)
                .await
                .len(),
            1
        );

        let remaining: (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM \"{}\".\"person\" WHERE id = $1",
            seeded.schema_name
        ))
        .bind(person_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("counting the destroyed person");
        assert_eq!(remaining.0, 0);

        let cached_first = cache
            .get_or_build(
                &seeded.workspace_id,
                version,
                &person_object,
                &maps.field_metadata_maps,
            )
            .expect("cached shape");
        assert!(Arc::ptr_eq(&person, &cached_first));

        let new_version = bump_metadata_version(&state.core_pool, seeded.workspace_id)
            .await
            .expect("bumping the metadata version");
        assert_eq!(new_version, version + 1);

        let rebuilt = cache
            .get_or_build(
                &seeded.workspace_id,
                new_version,
                &person_object,
                &maps.field_metadata_maps,
            )
            .expect("rebuilt shape");
        assert!(!Arc::ptr_eq(&person, &rebuilt));
        assert_eq!(cache.len(), 3);

        assert_eq!(cache.invalidate_workspace(&seeded.workspace_id), 3);
        assert!(cache.is_empty());

        let error = check_update_many_limit(QUERY_MAX_RECORDS + 1).expect_err("over the cap");
        assert_eq!(error.code(), "TOO_MANY_RECORDS_TO_UPDATE");

        let company_count: (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM \"{}\".\"company\" WHERE id = $1",
            seeded.schema_name
        ))
        .bind(company_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("counting the inserted company");
        assert_eq!(company_count.0, 1);

        sqlx::query(&format!("DROP SCHEMA \"{}\" CASCADE", seeded.schema_name))
            .execute(&state.core_pool)
            .await
            .expect("dropping the repository schema");
    }
}
