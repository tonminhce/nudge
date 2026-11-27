use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use sqlx::query::Query;
use sqlx::Column;
use sqlx::Row;
use sqlx::TypeInfo;
use sqlx::ValueRef;
use uuid::Uuid;

use super::RunnerError;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FlatEntityMaps;
use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::FlatObjectMetadata;
use crate::metadata::flat::RelationType;
use crate::metadata::names::compute_composite_column_name;
use crate::metadata::names::compute_relation_join_column_name;
use crate::query::filter::compile_filter;
use crate::query::filter::model::ClauseKind;
use crate::query::filter::model::CompiledFilter;
use crate::query::filter::model::CompositeSubFieldInfo;
use crate::query::filter::model::FilterKeyInfo;
use crate::query::filter::model::FilterSchema;
use crate::query::filter::model::WhereClause;
use crate::query::filter::Suffixes;
use crate::query::order::OrderByDirection;
use crate::query::repository::build_column_result_alias;
use crate::query::repository::quote_identifier;
use crate::query::repository::table_shape::build_workspace_table_shape;
use crate::query::repository::table_shape::ColumnShape;
use crate::query::repository::table_shape::RelationShape;
use crate::query::repository::table_shape::TableShape;

pub const QUERY_MAX_RECORDS: usize = 200;

pub const DEFAULT_PAGE_SIZE: usize = QUERY_MAX_RECORDS;

pub const NESTED_RELATION_QUERY_MAX_CONCURRENCY: usize = 4;

pub const QUERY_MAX_RECORDS_FROM_RELATION: usize = 60;

pub const COMMON_QUERY_COMPLEXITY_LIMIT: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniqueField {
    pub field_name: String,
    pub sub_field_name: Option<String>,
}

#[derive(Clone)]
pub struct ObjectModel {
    pub object: FlatObjectMetadata,
    pub shape: Arc<TableShape>,
    pub fields_by_name: HashMap<String, FlatFieldMetadata>,
    pub unique_field_groups: Vec<Vec<UniqueField>>,
}

pub struct RunnerSchema {
    objects_by_name: HashMap<String, ObjectModel>,
    objects_by_id: HashMap<Uuid, ObjectModel>,
    records_permissions: Option<crate::permissions::RoleRecordsPermissions>,
    mask_unreadable_fields: bool,
}

impl RunnerSchema {
    pub fn build(
        workspace_id: &Uuid,
        objects: &[FlatObjectMetadata],
        fields: &FlatEntityMaps<FlatFieldMetadata>,
        indexes: &FlatEntityMaps<crate::metadata::flat::FlatIndexMetadata>,
    ) -> Result<Self, RunnerError> {
        Self::build_inner(workspace_id, objects, fields, indexes, None, false)
    }

    pub fn build_with_permissions(
        workspace_id: &Uuid,
        objects: &[FlatObjectMetadata],
        fields: &FlatEntityMaps<FlatFieldMetadata>,
        indexes: &FlatEntityMaps<crate::metadata::flat::FlatIndexMetadata>,
        permissions: crate::permissions::RoleRecordsPermissions,
    ) -> Result<Self, RunnerError> {
        Self::build_inner(
            workspace_id,
            objects,
            fields,
            indexes,
            Some(permissions),
            false,
        )
    }

    pub fn build_for_rest_with_permissions(
        workspace_id: &Uuid,
        objects: &[FlatObjectMetadata],
        fields: &FlatEntityMaps<FlatFieldMetadata>,
        indexes: &FlatEntityMaps<crate::metadata::flat::FlatIndexMetadata>,
        permissions: crate::permissions::RoleRecordsPermissions,
    ) -> Result<Self, RunnerError> {
        Self::build_inner(
            workspace_id,
            objects,
            fields,
            indexes,
            Some(permissions),
            true,
        )
    }

    fn build_inner(
        workspace_id: &Uuid,
        objects: &[FlatObjectMetadata],
        fields: &FlatEntityMaps<FlatFieldMetadata>,
        indexes: &FlatEntityMaps<crate::metadata::flat::FlatIndexMetadata>,
        records_permissions: Option<crate::permissions::RoleRecordsPermissions>,
        mask_unreadable_fields: bool,
    ) -> Result<Self, RunnerError> {
        let mut objects_by_name = HashMap::new();
        let mut objects_by_id = HashMap::new();
        let mut unique_groups_by_object: HashMap<Uuid, Vec<Vec<UniqueField>>> = HashMap::new();

        for index in indexes.by_universal_identifier.values() {
            if !index.is_unique {
                continue;
            }

            let group = index
                .flat_index_field_metadatas
                .iter()
                .flat_map(|index_field| {
                    let Some(field) =
                        fields.find_by_id(&index_field.field_metadata_id.to_string())
                    else {
                        return Vec::new();
                    };

                    if let Some(sub_field_name) = index_field.sub_field_name.as_deref() {
                        return vec![UniqueField {
                            field_name: field.name.clone(),
                            sub_field_name: Some(sub_field_name.to_string()),
                        }];
                    }

                    if crate::workspace_migrations::column_defs::composite_types::is_composite_field_metadata_type(
                        crate::workspace_migrations::runner::ddl_field_type(field.field_type),
                    ) {
                        return crate::workspace_migrations::column_defs::composite_types::composite_properties(
                            crate::workspace_migrations::runner::ddl_field_type(field.field_type),
                        )
                        .iter()
                        .filter(|property| property.is_included_in_unique_constraint)
                        .map(|property| UniqueField {
                            field_name: field.name.clone(),
                            sub_field_name: Some(property.name.to_string()),
                        })
                        .collect::<Vec<_>>();
                    }

                    vec![UniqueField {
                        field_name: field.name.clone(),
                        sub_field_name: None,
                    }]
                })
                .collect::<Vec<_>>();

            if !group.is_empty() {
                unique_groups_by_object
                    .entry(index.object_metadata_id)
                    .or_default()
                    .push(group);
            }
        }

        for object in objects {
            let shape = Arc::new(build_workspace_table_shape(workspace_id, object, fields));
            let mut fields_by_name = HashMap::new();

            for field_id in &object.field_ids {
                if let Some(field) = fields.find_by_id(field_id) {
                    fields_by_name.insert(field.name.clone(), field.clone());
                }
            }

            let model = ObjectModel {
                object: object.clone(),
                shape,
                fields_by_name,
                unique_field_groups: unique_groups_by_object
                    .remove(&object.id)
                    .unwrap_or_default(),
            };

            objects_by_name.insert(object.name_singular.clone(), model.clone());
            objects_by_id.insert(object.id, model);
        }

        Ok(Self {
            objects_by_name,
            objects_by_id,
            records_permissions,
            mask_unreadable_fields,
        })
    }

    pub fn records_permissions(&self) -> Option<&crate::permissions::RoleRecordsPermissions> {
        self.records_permissions.as_ref()
    }

    pub fn masks_unreadable_fields(&self) -> bool {
        self.mask_unreadable_fields
    }

    pub fn is_field_unreadable(&self, object_metadata_id: Uuid, field_metadata_id: Uuid) -> bool {
        self.records_permissions
            .as_ref()
            .and_then(|permissions| permissions.object(object_metadata_id))
            .is_some_and(|object| {
                object
                    .unreadable_fields
                    .iter()
                    .any(|field_id| field_id == &field_metadata_id.to_string())
            })
    }

    pub fn select_read_columns(
        &self,
        model: &ObjectModel,
        selection: &Selection,
    ) -> Result<Vec<ColumnShape>, RunnerError> {
        let mut columns = select_columns(model, selection)?;
        if self.mask_unreadable_fields {
            columns.retain(|column| {
                !self.is_field_unreadable(model.object.id, column.field_metadata_id)
            });
        }
        Ok(columns)
    }

    pub fn object(&self, name_singular: &str) -> Result<&ObjectModel, RunnerError> {
        self.objects_by_name
            .get(name_singular)
            .ok_or_else(|| RunnerError::unknown_object(name_singular))
    }

    pub fn object_by_id(&self, id: &Uuid) -> Result<&ObjectModel, RunnerError> {
        self.objects_by_id
            .get(id)
            .ok_or_else(|| RunnerError::unknown_object(&id.to_string()))
    }

    pub fn relation(
        &self,
        object_name: &str,
        relation_field_name: &str,
    ) -> Result<(&ObjectModel, &RelationShape, &ObjectModel), RunnerError> {
        let parent = self.object(object_name)?;
        let relation = parent
            .shape
            .relation(relation_field_name)
            .ok_or_else(|| RunnerError::unknown_relation(relation_field_name))?;
        let target_id = relation
            .target_object_metadata_id
            .ok_or_else(|| RunnerError::unknown_relation(relation_field_name))?;
        let child = self.object_by_id(&target_id)?;

        Ok((parent, relation, child))
    }

    pub fn child_join_column(
        &self,
        object_name: &str,
        relation_field_name: &str,
    ) -> Result<String, RunnerError> {
        let (_, relation, child) = self.relation(object_name, relation_field_name)?;

        if let Some(join_column) = relation.join_column_name.as_deref() {
            return Ok(join_column.to_string());
        }

        let target_field_id = relation
            .target_field_metadata_id
            .ok_or_else(|| RunnerError::unknown_relation(relation_field_name))?;
        let reverse = child
            .shape
            .columns
            .iter()
            .find(|column| column.field_metadata_id == target_field_id)
            .ok_or_else(|| RunnerError::unknown_relation(relation_field_name))?;

        Ok(reverse.column_name.clone())
    }
}

pub struct ResolvedOrder {
    pub spec: crate::query::cursor::OrderByLeafSpec,
    pub expression: String,
    pub order: crate::query::order::OrderByClause,
}

pub struct Selection {
    pub field_names: Vec<String>,
}

impl Selection {
    pub fn all() -> Self {
        Self {
            field_names: Vec::new(),
        }
    }

    pub fn from_fields(field_names: Vec<String>) -> Self {
        Self { field_names }
    }
}

impl FilterSchema for RunnerSchema {
    fn lookup(&self, object_name: &str, key: &str) -> Option<FilterKeyInfo> {
        let model = self.objects_by_name.get(object_name)?;

        if let Some(column) = model.shape.column(key) {
            if column.composite_parent_field_name.is_none()
                && !matches!(
                    column.field_metadata_type,
                    FieldMetadataType::Relation | FieldMetadataType::MorphRelation
                )
            {
                return Some(FilterKeyInfo::Scalar(column.field_metadata_type));
            }

            if column.composite_parent_field_name.is_none()
                && matches!(column.field_metadata_type, FieldMetadataType::Relation)
            {
                return Some(FilterKeyInfo::Scalar(FieldMetadataType::Uuid));
            }
        }

        let field = model.fields_by_name.get(key)?;
        let relation_shape = model.shape.relation(key);

        if let Some(relation) = relation_shape {
            let target_id = relation.target_object_metadata_id?;
            let child = self.objects_by_id.get(&target_id)?;
            let to_many = relation.relation_type != RelationType::ManyToOne;
            let join_column = relation
                .join_column_name
                .clone()
                .unwrap_or_else(|| compute_relation_join_column_name(key));

            return Some(FilterKeyInfo::Relation {
                child_object: child.object.name_singular.clone(),
                child_alias: format!("{}_{}", object_name, key),
                to_many,
                join_column,
            });
        }

        if crate::workspace_migrations::column_defs::composite_types::is_composite_field_metadata_type(
            crate::workspace_migrations::runner::ddl_field_type(field.field_type),
        ) {
            let sub_fields = model
                .shape
                .columns
                .iter()
                .filter_map(|column| {
                    column.composite_parent_field_name.as_deref().and_then(|parent| {
                        if parent != key {
                            return None;
                        }

                        let suffix = column
                            .column_name
                            .strip_prefix(key)
                            .unwrap_or(&column.column_name);

                        Some(CompositeSubFieldInfo {
                            name: lowercase_first(suffix),
                            field_type: column.field_metadata_type,
                        })
                    })
                })
                .collect();

            return Some(FilterKeyInfo::Composite {
                field_type: field.field_type,
                sub_fields,
            });
        }

        Some(FilterKeyInfo::Scalar(field.field_type))
    }
}

pub fn compile_runner_filter(
    schema: &RunnerSchema,
    object_name: &str,
    filter: Option<&Value>,
    suffixes: &mut Suffixes,
) -> Result<Option<CompiledFilter>, RunnerError> {
    let Some(filter) = filter else {
        return Ok(None);
    };

    compile_filter(filter, object_name, schema, suffixes).map_err(RunnerError::from_filter)
}

pub fn resolve_where_clauses(
    schema: &RunnerSchema,
    object_name: &str,
    alias: &str,
    clauses: &[WhereClause],
) -> Result<String, RunnerError> {
    let mut rendered = String::new();

    for (index, clause) in clauses.iter().enumerate() {
        let sql = resolve_clause(schema, object_name, alias, clause)?;

        if index == 0 {
            rendered.push_str(&sql);
        } else {
            rendered.push_str(match clause.operator {
                crate::query::filter::JoinOp::And => " AND ",
                crate::query::filter::JoinOp::Or => " OR ",
            });
            rendered.push_str(&sql);
        }
    }

    Ok(rendered)
}

fn resolve_clause(
    schema: &RunnerSchema,
    object_name: &str,
    alias: &str,
    clause: &WhereClause,
) -> Result<String, RunnerError> {
    match &clause.kind {
        ClauseKind::Sql(sql) => Ok(format!("({sql})")),
        ClauseKind::Group { negated, clauses } => {
            let inner = resolve_where_clauses(schema, object_name, alias, clauses)?;

            if inner.is_empty() {
                return Ok(String::from("TRUE"));
            }

            if *negated {
                Ok(format!("NOT ({inner})"))
            } else {
                Ok(format!("({inner})"))
            }
        }
        ClauseKind::ExistsToken {
            relation,
            alias: child_alias,
            inner,
        } => {
            let (_, relation_shape, child) = schema.relation(object_name, relation)?;
            let child_join_column = schema.child_join_column(object_name, relation)?;
            let inner_sql =
                resolve_where_clauses(schema, &child.object.name_singular, child_alias, inner)?;
            let live_predicate = if child.shape.has_deleted_at_column {
                Some(format!(
                    "{}.{} IS NULL",
                    quote_identifier(child_alias)?,
                    quote_identifier("deletedAt")?
                ))
            } else {
                None
            };
            let mut conditions = vec![format!(
                "{}.{} = {}.{}",
                quote_identifier(child_alias)?,
                quote_identifier(&child_join_column)?,
                quote_identifier(alias)?,
                quote_identifier("id")?
            )];

            if let Some(predicate) = live_predicate {
                conditions.push(predicate);
            }

            if !inner_sql.is_empty() {
                conditions.push(format!("({inner_sql})"));
            }

            let _ = relation_shape;

            Ok(format!(
                "EXISTS (SELECT 1 FROM {}.{} AS {} WHERE {})",
                quote_identifier(&child.shape.schema_name)?,
                quote_identifier(&child.shape.table_name)?,
                quote_identifier(child_alias)?,
                conditions.join(" AND ")
            ))
        }
    }
}

pub fn select_columns(
    model: &ObjectModel,
    selection: &Selection,
) -> Result<Vec<ColumnShape>, RunnerError> {
    let mut selected: Vec<ColumnShape> = Vec::new();

    if selection.field_names.is_empty() {
        selected.extend(model.shape.columns.iter().cloned());
    } else {
        for field_name in &selection.field_names {
            if let Some(column) = model.shape.column(field_name) {
                if column.composite_parent_field_name.is_none() {
                    selected.push(column.clone());
                    continue;
                }
            }

            let mut composite = model
                .shape
                .columns
                .iter()
                .filter(|column| {
                    column
                        .composite_parent_field_name
                        .as_deref()
                        .is_some_and(|parent| parent == field_name)
                })
                .cloned()
                .collect::<Vec<_>>();

            if composite.is_empty() {
                if let Some(relation) = model.shape.relation(field_name) {
                    if let Some(join_column) = relation.join_column_name.as_deref() {
                        if let Some(column) = model.shape.column(join_column) {
                            selected.push(column.clone());
                            continue;
                        }
                    }
                }

                return Err(RunnerError::unknown_column(
                    field_name,
                    &model.object.name_singular,
                ));
            }

            selected.append(&mut composite);
        }
    }

    Ok(selected)
}

pub fn render_select_list(
    alias: &str,
    columns: &[(ColumnShape, String, String)],
    order_scan_values: &[(String, String)],
) -> Result<String, RunnerError> {
    let mut parts = Vec::with_capacity(columns.len() + order_scan_values.len());

    for (column, result_alias, _) in columns {
        let expression = match column.field_metadata_type {
            FieldMetadataType::Numeric => {
                format!("{}::text", quoted_column(alias, &column.column_name)?)
            }
            FieldMetadataType::Text
            | FieldMetadataType::Select
            | FieldMetadataType::Rating
            | FieldMetadataType::TsVector => {
                format!("{}::text", quoted_column(alias, &column.column_name)?)
            }
            _ => quoted_column(alias, &column.column_name)?,
        };

        parts.push(format!(
            "{} AS {}",
            expression,
            quote_identifier(result_alias)?
        ));
    }

    for (expression, result_alias) in order_scan_values {
        parts.push(format!(
            "{expression} AS {}",
            quote_identifier(result_alias)?
        ));
    }

    Ok(parts.join(", "))
}

pub fn quoted_column(alias: &str, column_name: &str) -> Result<String, RunnerError> {
    Ok(format!(
        "{}.{}",
        quote_identifier(alias)?,
        quote_identifier(column_name)?
    ))
}

pub fn column_result_alias(alias: &str, column_name: &str) -> String {
    build_column_result_alias(alias, column_name)
}

pub fn order_expression(
    _model: &ObjectModel,
    alias: &str,
    column_name: &str,
    field_type: FieldMetadataType,
) -> Result<String, RunnerError> {
    let column = quoted_column(alias, column_name)?;

    if crate::query::order::should_use_case_insensitive_order(field_type) {
        return Ok(format!("LOWER({column})"));
    }

    if crate::query::order::should_cast_to_text(field_type) {
        return Ok(format!("{column}::text"));
    }

    Ok(column)
}

pub fn direction(direction: &str) -> Result<OrderByDirection, RunnerError> {
    OrderByDirection::parse(direction).ok_or_else(|| RunnerError::invalid_direction(direction))
}

pub fn composite_column(
    model: &ObjectModel,
    parent: &str,
    sub_field: &str,
) -> Result<String, RunnerError> {
    let column_name = compute_composite_column_name(parent, sub_field);

    if model.shape.column(&column_name).is_some() {
        return Ok(column_name);
    }

    Err(RunnerError::unknown_sub_field(
        parent,
        sub_field,
        Vec::new(),
    ))
}

pub fn lowercase_first(value: &str) -> String {
    let mut characters = value.chars();
    match characters.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + characters.as_str(),
    }
}

pub async fn execute_json_rows(
    pool: &sqlx::PgPool,
    sql: &str,
    parameters: &[Value],
) -> Result<Vec<serde_json::Map<String, Value>>, RunnerError> {
    let mut query = sqlx::query(sql);

    for value in parameters {
        query = bind_json_value(query, value)?;
    }

    let rows = query.fetch_all(pool).await.map_err(RunnerError::database)?;

    rows.iter().map(row_to_json).collect()
}

pub fn bind_json_value<'q>(
    query: Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    value: &'q Value,
) -> Result<Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>, RunnerError> {
    Ok(match value {
        Value::Null => query.bind(Option::<String>::None),
        Value::Bool(value) => query.bind(*value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                query.bind(value)
            } else if let Some(value) = value.as_u64() {
                query.bind(i64::try_from(value).map_err(|_| RunnerError::invalid_parameter())?)
            } else {
                query.bind(value.as_f64().unwrap_or_default())
            }
        }
        Value::String(value) => query.bind(value.as_str()),
        Value::Array(items) => {
            let items = items
                .iter()
                .map(|item| item.as_str().unwrap_or_default().to_string())
                .collect::<Vec<_>>();
            query.bind(items)
        }
        Value::Object(_) => query.bind(value.to_string()),
    })
}

pub fn row_to_json(
    row: &sqlx::postgres::PgRow,
) -> Result<serde_json::Map<String, Value>, RunnerError> {
    let mut record = serde_json::Map::new();

    for column in row.columns() {
        let name = column.name();
        let value = json_cell(row, name)?;
        record.insert(name.to_string(), value);
    }

    Ok(record)
}

pub(crate) fn json_cell(row: &sqlx::postgres::PgRow, name: &str) -> Result<Value, RunnerError> {
    let raw = row.try_get_raw(name).map_err(RunnerError::database)?;

    if raw.is_null() {
        return Ok(Value::Null);
    }

    let kind = raw.type_info().name().to_string();
    let value = match kind.as_str() {
        "BOOL" => row.try_get::<bool, _>(name).map(Value::Bool),
        "INT2" => row.try_get::<i16, _>(name).map(|v| number(v as i64)),
        "INT4" => row.try_get::<i32, _>(name).map(|v| number(v as i64)),
        "INT8" => row.try_get::<i64, _>(name).map(number),
        "FLOAT4" | "FLOAT8" => row.try_get::<f64, _>(name).map(number_f64),
        "TEXT" | "VARCHAR" | "CHAR" | "NAME" | "CITEXT" | "UNKNOWN" => {
            row.try_get::<String, _>(name).map(Value::String)
        }
        "UUID" => row
            .try_get::<Uuid, _>(name)
            .map(|v| Value::String(v.to_string())),
        "DATE" => row
            .try_get::<chrono::NaiveDate, _>(name)
            .map(|v| Value::String(v.to_string())),
        "TIME" => row
            .try_get::<chrono::NaiveTime, _>(name)
            .map(|v| Value::String(v.to_string())),
        "TIMESTAMP" => row
            .try_get::<chrono::NaiveDateTime, _>(name)
            .map(|v| Value::String(v.to_string())),
        "TIMESTAMPTZ" => row
            .try_get::<chrono::DateTime<chrono::Utc>, _>(name)
            .map(|v| Value::String(v.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))),
        "JSON" | "JSONB" => row.try_get::<Value, _>(name),
        "BYTEA" => row.try_get::<Vec<u8>, _>(name).map(|v| {
            Value::String(
                v.iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
            )
        }),
        "TEXT[]" => row
            .try_get::<Vec<String>, _>(name)
            .map(|items| Value::Array(items.into_iter().map(Value::String).collect())),
        "UUID[]" => row.try_get::<Vec<Uuid>, _>(name).map(|items| {
            Value::Array(
                items
                    .into_iter()
                    .map(|item| Value::String(item.to_string()))
                    .collect(),
            )
        }),
        "INT4[]" | "INT8[]" => row
            .try_get::<Vec<i64>, _>(name)
            .map(|items| Value::Array(items.into_iter().map(number).collect())),
        _ => row.try_get::<String, _>(name).map(Value::String),
    };

    value.map_err(RunnerError::database)
}

fn number(value: i64) -> Value {
    serde_json::Number::from(value).into()
}

fn number_f64(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 9.007_199_254_740_992e15 {
        return serde_json::Number::from(value as i64).into();
    }

    serde_json::Number::from_f64(value)
        .unwrap_or_else(|| serde_json::Number::from(0))
        .into()
}

pub fn validate_complexity(
    selected_field_count: usize,
    relation_field_count: usize,
) -> Result<(), RunnerError> {
    if selected_field_count.saturating_add(relation_field_count) > COMMON_QUERY_COMPLEXITY_LIMIT {
        return Err(RunnerError::too_complex());
    }

    Ok(())
}
