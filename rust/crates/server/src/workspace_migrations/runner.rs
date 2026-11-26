#![allow(dead_code)]

use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;

use super::column_defs;
use super::column_defs::composite_types;
use super::column_defs::composite_types::CompositeProperty;
use super::column_defs::generate_column_definitions;
use super::column_defs::FlatFieldColumnInput;
use super::column_defs::GeneratedType;
use super::column_defs::WorkspaceSchemaColumnDefinition;
use super::ddl::column;
use super::ddl::enum_types;
use super::ddl::error::DdlError;
use super::ddl::foreign_key;
use super::ddl::foreign_key::WorkspaceSchemaForeignKeyDefinition;
use super::ddl::index;
use super::ddl::index::WorkspaceSchemaIndexDefinition;
use super::ddl::table;
use super::deferred;
use super::deferred::DeferredOperationInput;
use super::tsvector::compute_search_vector_as_expression;
use super::tsvector::SearchVectorTargetField;
use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalCreateAction;
use crate::metadata::actions::UniversalDeleteAction;
use crate::metadata::actions::UniversalFlatEntity;
use crate::metadata::actions::UniversalUpdateAction;
use crate::metadata::actions::WorkspaceMigration;
use crate::metadata::actions::WorkspaceMigrationActionType;
use crate::metadata::flat::FieldMetadataType as FlatFieldMetadataType;
use crate::metadata::flat::IndexType;
use crate::metadata::flat::UniversalFlatFieldMetadata;
use crate::metadata::flat::UniversalFlatFieldMetadataOption;
use crate::metadata::flat::UniversalFlatIndexMetadata;
use crate::metadata::flat::UniversalFlatObjectMetadata;
use crate::metadata::flat::UniversalFlatSearchFieldMetadata;
use crate::metadata::names::compute_composite_column_name;
use crate::metadata::names::compute_object_target_table;
use crate::metadata::names::compute_postgres_enum_name;
use crate::metadata::names::compute_relation_join_column_name;
use crate::state::AppState;

pub const DDL_LOCK_TIMEOUT: &str = "8s";
pub const DEFAULT_RUN_BY_VERSION: &str = "nudge-0.1.0";

pub const SCHEMA_AFFECTING_METADATA_NAMES: [MetadataName; 4] = [
    MetadataName::ObjectMetadata,
    MetadataName::FieldMetadata,
    MetadataName::FieldMetadataOption,
    MetadataName::Index,
];

const SEARCH_VECTOR_FIELD_NAME: &str = "searchVector";

pub const DEPRECATED_TARGET_TABLE_NAME: &str = "DEPRECATED";

#[derive(Debug)]
pub struct RunnerError {
    pub code: &'static str,
    pub message: String,
    pub action_index: Option<usize>,
}

impl RunnerError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            action_index: None,
        }
    }

    fn at(mut self, action_index: usize) -> Self {
        self.action_index = Some(action_index);
        self
    }

    pub fn ddl_locked() -> Self {
        Self::new(
            "DDL_LOCKED",
            "Workspace schema DDL is locked on this instance",
        )
    }

    pub fn deferred_in_progress() -> Self {
        Self::new(
            "DEFERRED_WORKSPACE_MIGRATION_ACTIONS_IN_PROGRESS",
            "Schema-affecting deferred workspace operations are in progress",
        )
    }

    pub fn application_not_found(universal_identifier: &str) -> Self {
        Self::new(
            "APPLICATION_NOT_FOUND",
            format!("Application {universal_identifier} not found"),
        )
    }

    pub fn workspace_not_found(workspace_id: Uuid) -> Self {
        Self::new(
            "WORKSPACE_NOT_FOUND",
            format!("Workspace {workspace_id} not found"),
        )
    }

    pub fn execution_failed(message: impl Into<String>) -> Self {
        Self::new("EXECUTION_FAILED", message)
    }

    pub fn unsupported_metadata_name(metadata_name: MetadataName) -> Self {
        Self::new(
            "UNSUPPORTED_METADATA_NAME",
            format!("No runner handler for metadata name {metadata_name:?}"),
        )
    }
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for RunnerError {}

impl From<DdlError> for RunnerError {
    fn from(error: DdlError) -> Self {
        RunnerError::execution_failed(error.to_string())
    }
}

impl From<sqlx::Error> for RunnerError {
    fn from(error: sqlx::Error) -> Self {
        RunnerError::execution_failed(error.to_string())
    }
}

pub fn is_schema_affecting(metadata_name: MetadataName) -> bool {
    SCHEMA_AFFECTING_METADATA_NAMES.contains(&metadata_name)
}

pub fn migration_is_schema_affecting(migration: &WorkspaceMigration) -> bool {
    migration
        .actions
        .iter()
        .any(|action| is_schema_affecting(action.metadata_name()))
}

pub struct RunnerInput<'a> {
    pub workspace_id: Uuid,
    pub migration: &'a WorkspaceMigration,
    pub should_defer_actions: bool,
}

#[derive(Debug, Default)]
pub struct PreallocatedIds {
    by_name: std::collections::HashMap<MetadataName, std::collections::HashMap<String, Uuid>>,
}

impl PreallocatedIds {
    pub fn get(&self, metadata_name: MetadataName, universal_identifier: &str) -> Option<Uuid> {
        self.by_name
            .get(&metadata_name)
            .and_then(|by_identifier| by_identifier.get(universal_identifier))
            .copied()
    }

    fn register(
        &mut self,
        metadata_name: MetadataName,
        universal_identifier: &str,
        id: Option<Uuid>,
    ) {
        let Some(id) = id else { return };

        self.by_name
            .entry(metadata_name)
            .or_default()
            .insert(universal_identifier.to_string(), id);
    }
}

pub fn build_preallocated_ids(actions: &[UniversalAction]) -> PreallocatedIds {
    let mut ids = PreallocatedIds::default();

    for action in actions {
        let UniversalAction::Create(create) = action else {
            continue;
        };

        if let Some(entity) = &create.flat_entity {
            let id = create
                .id
                .as_deref()
                .and_then(|raw| Uuid::parse_str(raw).ok());
            ids.register(create.metadata_name, entity.universal_identifier(), id);

            if create.metadata_name == MetadataName::ObjectMetadata {
                if let Some(field_ids) = &create.field_id_by_universal_identifier {
                    for field in &create.universal_flat_field_metadatas {
                        let id = field_ids
                            .get(&field.universal_identifier)
                            .and_then(|raw| Uuid::parse_str(raw).ok());
                        ids.register(MetadataName::FieldMetadata, &field.universal_identifier, id);
                    }
                }
            }

            if create.metadata_name == MetadataName::FieldMetadata {
                if let Some(related) = &create.related_universal_flat_field_metadata {
                    let id = create
                        .related_field_id
                        .as_deref()
                        .and_then(|raw| Uuid::parse_str(raw).ok());
                    ids.register(
                        MetadataName::FieldMetadata,
                        &related.universal_identifier,
                        id,
                    );
                }
            }
        }
    }

    ids
}

#[derive(Debug, Default)]
pub struct RunnerOutcome {
    pub deferred_operations: usize,
    pub metadata_version: Option<i32>,
}

pub struct WorkspaceRuntime {
    pub workspace_id: Uuid,
    pub schema_name: String,
    pub application_id: Uuid,
    pub application_universal_identifier: String,
}

pub async fn run_workspace_migration(
    state: &AppState,
    input: RunnerInput<'_>,
) -> Result<RunnerOutcome, RunnerError> {
    if state.config.workspace_schema_ddl_locked {
        return Err(RunnerError::ddl_locked());
    }

    let schema_affecting = migration_is_schema_affecting(input.migration);

    if input.should_defer_actions
        && schema_affecting
        && deferred::has_in_flight_schema_affecting(state, input.workspace_id).await?
    {
        return Err(RunnerError::deferred_in_progress());
    }

    let runtime = resolve_workspace_runtime(
        &state.core_pool,
        input.workspace_id,
        &input.migration.application_universal_identifier,
    )
    .await?;

    let mut transaction = state.core_pool.begin().await?;

    sqlx::query(&format!("SET LOCAL lock_timeout = '{}'", DDL_LOCK_TIMEOUT))
        .execute(&mut *transaction)
        .await?;

    let preallocated = build_preallocated_ids(&input.migration.actions);
    let mut deferred_inputs: Vec<DeferredOperationInput> = Vec::new();

    {
        let connection: &mut PgConnection = &mut transaction;

        for (action_index, action) in input.migration.actions.iter().enumerate() {
            let deferred_input = apply_action(
                connection,
                &runtime,
                action,
                input.should_defer_actions,
                &preallocated,
            )
            .await
            .map_err(|error| error.at(action_index))?;

            if let Some(deferred_input) = deferred_input {
                deferred_inputs.push(deferred_input);
            }
        }
    }

    if !deferred_inputs.is_empty() {
        deferred::persist_many(&mut transaction, input.workspace_id, &deferred_inputs).await?;
    }

    transaction.commit().await?;

    crate::metadata::cache::invalidate_workspace_maps(state, input.workspace_id).await;

    let metadata_version = if schema_affecting {
        Some(bump_metadata_version(&state.core_pool, input.workspace_id).await?)
    } else {
        None
    };

    if !deferred_inputs.is_empty() && !input.should_defer_actions {
        deferred::run_pending_inline(state, input.workspace_id).await?;
    }

    Ok(RunnerOutcome {
        deferred_operations: deferred_inputs.len(),
        metadata_version,
    })
}

pub async fn resolve_workspace_runtime(
    core_pool: &sqlx::PgPool,
    workspace_id: Uuid,
    application_universal_identifier: &str,
) -> Result<WorkspaceRuntime, RunnerError> {
    let workspace: Option<(Option<String>, Option<Uuid>)> = sqlx::query_as(
        "SELECT \"databaseSchema\", \"workspaceCustomApplicationId\" \
         FROM core.\"workspace\" WHERE id = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(workspace_id)
    .fetch_optional(core_pool)
    .await?;

    let (schema_name, custom_application_id) =
        workspace.ok_or_else(|| RunnerError::workspace_not_found(workspace_id))?;

    let schema_name = schema_name.ok_or_else(|| {
        RunnerError::execution_failed(format!("Workspace {workspace_id} has no database schema"))
    })?;

    let application = match Uuid::parse_str(application_universal_identifier) {
        Ok(identifier_uuid) => {
            let row: Option<(Uuid, Uuid)> = sqlx::query_as(
                "SELECT id, \"universalIdentifier\" FROM core.\"application\" \
                 WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
            )
            .bind(workspace_id)
            .bind(identifier_uuid)
            .fetch_optional(core_pool)
            .await?;

            row.map(|(id, identifier)| (id, identifier.to_string()))
        }
        Err(_) => None,
    };

    let (application_id, application_universal_identifier) = match application {
        Some(row) => row,
        None => {
            let custom_id = custom_application_id.ok_or_else(|| {
                RunnerError::application_not_found(application_universal_identifier)
            })?;

            let fallback: Option<(Uuid, Uuid)> = sqlx::query_as(
                "SELECT id, \"universalIdentifier\" FROM core.\"application\" WHERE id = $1",
            )
            .bind(custom_id)
            .fetch_optional(core_pool)
            .await?;

            fallback
                .map(|(id, identifier)| (id, identifier.to_string()))
                .ok_or_else(|| {
                    RunnerError::application_not_found(application_universal_identifier)
                })?
        }
    };

    Ok(WorkspaceRuntime {
        workspace_id,
        schema_name,
        application_id,
        application_universal_identifier,
    })
}

pub async fn bump_metadata_version(
    core_pool: &sqlx::PgPool,
    workspace_id: Uuid,
) -> Result<i32, RunnerError> {
    let row: (i32,) = sqlx::query_as(
        "UPDATE core.\"workspace\" SET \"metadataVersion\" = \"metadataVersion\" + 1, \
         \"updatedAt\" = now() WHERE id = $1 RETURNING \"metadataVersion\"",
    )
    .bind(workspace_id)
    .fetch_one(core_pool)
    .await?;

    Ok(row.0)
}

async fn apply_action(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    action: &UniversalAction,
    should_defer_actions: bool,
    preallocated: &PreallocatedIds,
) -> Result<Option<DeferredOperationInput>, RunnerError> {
    match (action.metadata_name(), action.action_type()) {
        (MetadataName::ObjectMetadata, WorkspaceMigrationActionType::Create) => {
            apply_object_create(connection, runtime, expect_create(action)?, preallocated).await?;
            Ok(None)
        }
        (MetadataName::ObjectMetadata, WorkspaceMigrationActionType::Update) => {
            apply_object_update(connection, runtime, expect_update(action)?).await?;
            Ok(None)
        }
        (MetadataName::ObjectMetadata, WorkspaceMigrationActionType::Delete) => {
            apply_object_delete(connection, runtime, expect_delete(action)?).await?;
            Ok(None)
        }
        (MetadataName::FieldMetadata, WorkspaceMigrationActionType::Create) => {
            apply_field_create(
                connection,
                runtime,
                expect_create(action)?,
                should_defer_actions,
                preallocated,
            )
            .await
        }
        (MetadataName::FieldMetadata, WorkspaceMigrationActionType::Update) => {
            apply_field_update(connection, runtime, expect_update(action)?).await?;
            Ok(None)
        }
        (MetadataName::FieldMetadata, WorkspaceMigrationActionType::Delete) => {
            apply_field_delete(connection, runtime, expect_delete(action)?).await?;
            Ok(None)
        }
        (MetadataName::Index, WorkspaceMigrationActionType::Create) => {
            apply_index_create(
                connection,
                runtime,
                expect_create(action)?,
                should_defer_actions,
                preallocated,
            )
            .await
        }
        (MetadataName::Index, WorkspaceMigrationActionType::Update) => {
            apply_index_update(connection, runtime, expect_update(action)?).await?;
            Ok(None)
        }
        (MetadataName::Index, WorkspaceMigrationActionType::Delete) => {
            apply_index_delete(connection, runtime, expect_delete(action)?).await?;
            Ok(None)
        }
        (MetadataName::FieldMetadataOption, WorkspaceMigrationActionType::Create) => {
            apply_option_create(connection, runtime, expect_create(action)?).await?;
            Ok(None)
        }
        (MetadataName::FieldMetadataOption, WorkspaceMigrationActionType::Update) => {
            apply_option_update(connection, runtime, expect_update(action)?).await?;
            Ok(None)
        }
        (MetadataName::FieldMetadataOption, WorkspaceMigrationActionType::Delete) => {
            apply_option_delete(connection, runtime, expect_delete(action)?).await?;
            Ok(None)
        }
        (MetadataName::SearchFieldMetadata, WorkspaceMigrationActionType::Create) => {
            apply_search_field_create(connection, runtime, expect_create(action)?, preallocated)
                .await?;
            Ok(None)
        }
        (MetadataName::SearchFieldMetadata, WorkspaceMigrationActionType::Update) => {
            apply_search_field_update(connection, runtime, expect_update(action)?).await?;
            Ok(None)
        }
        (MetadataName::SearchFieldMetadata, WorkspaceMigrationActionType::Delete) => {
            apply_search_field_delete(connection, runtime, expect_delete(action)?).await?;
            Ok(None)
        }
        (metadata_name, _) => Err(RunnerError::unsupported_metadata_name(metadata_name)),
    }
}

fn expect_create(action: &UniversalAction) -> Result<&UniversalCreateAction, RunnerError> {
    match action {
        UniversalAction::Create(create) => Ok(create),
        _ => Err(RunnerError::execution_failed("expected a create action")),
    }
}

fn expect_update(action: &UniversalAction) -> Result<&UniversalUpdateAction, RunnerError> {
    match action {
        UniversalAction::Update(update) => Ok(update),
        _ => Err(RunnerError::execution_failed("expected an update action")),
    }
}

fn expect_delete(action: &UniversalAction) -> Result<&UniversalDeleteAction, RunnerError> {
    match action {
        UniversalAction::Delete(delete) => Ok(delete),
        _ => Err(RunnerError::execution_failed("expected a delete action")),
    }
}

fn object_entity(
    create: &UniversalCreateAction,
) -> Result<&UniversalFlatObjectMetadata, RunnerError> {
    match create.flat_entity.as_ref() {
        Some(UniversalFlatEntity::ObjectMetadata(object)) => Ok(object),
        _ => Err(RunnerError::execution_failed(
            "object action is missing its flat entity",
        )),
    }
}

fn field_entity(
    action_entity: Option<&UniversalFlatEntity>,
) -> Result<&UniversalFlatFieldMetadata, RunnerError> {
    match action_entity {
        Some(UniversalFlatEntity::FieldMetadata(field)) => Ok(field),
        _ => Err(RunnerError::execution_failed(
            "field action is missing its flat entity",
        )),
    }
}

fn index_entity(
    action_entity: Option<&UniversalFlatEntity>,
) -> Result<&UniversalFlatIndexMetadata, RunnerError> {
    match action_entity {
        Some(UniversalFlatEntity::IndexMetadata(index)) => Ok(index),
        _ => Err(RunnerError::execution_failed(
            "index action is missing its flat entity",
        )),
    }
}

fn option_entity(
    action_entity: Option<&UniversalFlatEntity>,
) -> Result<&UniversalFlatFieldMetadataOption, RunnerError> {
    match action_entity {
        Some(UniversalFlatEntity::FieldMetadataOption(option)) => Ok(option),
        _ => Err(RunnerError::execution_failed(
            "option action is missing its flat entity",
        )),
    }
}

fn search_field_entity(
    action_entity: Option<&UniversalFlatEntity>,
) -> Result<&UniversalFlatSearchFieldMetadata, RunnerError> {
    match action_entity {
        Some(UniversalFlatEntity::SearchFieldMetadata(search_field)) => Ok(search_field),
        _ => Err(RunnerError::execution_failed(
            "search field action is missing its flat entity",
        )),
    }
}

pub fn universal_uuid(raw: &str) -> Result<Uuid, RunnerError> {
    Uuid::parse_str(raw).map_err(|error| {
        RunnerError::execution_failed(format!("invalid identifier {raw}: {error}"))
    })
}

pub async fn lookup_object_id(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    universal_identifier: &str,
) -> Result<Uuid, RunnerError> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"objectMetadata\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(runtime.workspace_id)
    .bind(universal_uuid(universal_identifier)?)
    .fetch_optional(&mut *connection)
    .await?;

    row.map(|(id,)| id).ok_or_else(|| {
        RunnerError::execution_failed(format!("object metadata {universal_identifier} not found"))
    })
}

pub struct FieldRow {
    pub id: Uuid,
    pub object_metadata_id: Uuid,
    pub name: String,
    pub label: String,
    pub field_type: FlatFieldMetadataType,
    pub is_nullable: bool,
    pub default_value: Option<Value>,
    pub settings: Option<Value>,
    pub relation_target_object_metadata_id: Option<Uuid>,
    pub universal_identifier: String,
    pub morph_id: Option<Uuid>,
}

type FieldRowTuple = (
    Uuid,
    Uuid,
    String,
    String,
    String,
    bool,
    Option<Value>,
    Option<Value>,
    Option<Uuid>,
    String,
    Option<Uuid>,
);

pub async fn lookup_field_row(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    universal_identifier: &str,
) -> Result<Option<FieldRow>, RunnerError> {
    let row: Option<FieldRowTuple> = sqlx::query_as(
        "SELECT id, \"objectMetadataId\", name, label, type, COALESCE(\"isNullable\", true), \
         \"defaultValue\", settings, \"relationTargetObjectMetadataId\", \
         \"universalIdentifier\"::text, \"morphId\" \
         FROM core.\"fieldMetadata\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(runtime.workspace_id)
    .bind(universal_uuid(universal_identifier)?)
    .fetch_optional(&mut *connection)
    .await?;

    Ok(row.and_then(
        |(
            id,
            object_id,
            name,
            label,
            field_type,
            is_nullable,
            default_value,
            settings,
            relation_target_object_metadata_id,
            universal_identifier,
            morph_id,
        )| {
            FlatFieldMetadataType::parse(&field_type).map(|parsed| FieldRow {
                id,
                object_metadata_id: object_id,
                name,
                label,
                field_type: parsed,
                is_nullable,
                default_value,
                settings,
                relation_target_object_metadata_id,
                universal_identifier,
                morph_id,
            })
        },
    ))
}

pub fn is_morph_or_relation_field_type(field_type: FlatFieldMetadataType) -> bool {
    matches!(
        field_type,
        FlatFieldMetadataType::Relation | FlatFieldMetadataType::MorphRelation
    )
}

pub fn is_morph_relation_head_row(row: &FieldRow) -> bool {
    row.field_type == FlatFieldMetadataType::MorphRelation
        && row.morph_id.is_some()
        && row.morph_id.map(|id| id.to_string()) == Some(row.universal_identifier.clone())
}

pub async fn object_table_name(
    connection: &mut PgConnection,
    object_id: Uuid,
) -> Result<String, RunnerError> {
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT om.\"nameSingular\", a.\"universalIdentifier\"::text \
         FROM core.\"objectMetadata\" om \
         JOIN core.\"application\" a ON a.id = om.\"applicationId\" \
         WHERE om.id = $1",
    )
    .bind(object_id)
    .fetch_optional(&mut *connection)
    .await?;

    row.map(|(name_singular, application_universal_identifier)| {
        compute_object_target_table(&name_singular, &application_universal_identifier)
    })
    .ok_or_else(|| RunnerError::execution_failed(format!("object metadata {object_id} not found")))
}

pub async fn search_vector_expression(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    object_id: Uuid,
) -> Result<String, RunnerError> {
    let rows: Vec<(String, String, f64, String)> = sqlx::query_as(
        "SELECT f.name, f.type, s.\"position\", s.\"universalIdentifier\"::text \
         FROM core.\"searchFieldMetadata\" s \
         JOIN core.\"fieldMetadata\" f ON f.id = s.\"fieldMetadataId\" \
         WHERE s.\"workspaceId\" = $1 AND s.\"objectMetadataId\" = $2 \
         ORDER BY s.\"position\" ASC, s.\"universalIdentifier\" ASC",
    )
    .bind(runtime.workspace_id)
    .bind(object_id)
    .fetch_all(&mut *connection)
    .await?;

    let target_fields: Vec<SearchVectorTargetField> = rows
        .into_iter()
        .filter_map(|(name, field_type, position, sort_key)| {
            Some(SearchVectorTargetField {
                name,
                field_type: ddl_field_type(FlatFieldMetadataType::parse(&field_type)?),
                position: position as i64,
                sort_key,
            })
        })
        .collect();

    compute_search_vector_as_expression(&target_fields)
        .map_err(|error| RunnerError::execution_failed(error.to_string()))
}

pub fn ddl_field_type(field_type: FlatFieldMetadataType) -> super::column_defs::FieldMetadataType {
    use super::column_defs::FieldMetadataType as Ddl;
    use FlatFieldMetadataType as Flat;

    match field_type {
        Flat::Actor => Ddl::Actor,
        Flat::Address => Ddl::Address,
        Flat::Array => Ddl::Array,
        Flat::Boolean => Ddl::Boolean,
        Flat::Currency => Ddl::Currency,
        Flat::Date => Ddl::Date,
        Flat::DateTime => Ddl::DateTime,
        Flat::Emails => Ddl::Emails,
        Flat::Files => Ddl::Files,
        Flat::FullName => Ddl::FullName,
        Flat::Links => Ddl::Links,
        Flat::MorphRelation => Ddl::MorphRelation,
        Flat::MultiSelect => Ddl::MultiSelect,
        Flat::Number => Ddl::Number,
        Flat::Numeric => Ddl::Numeric,
        Flat::Phones => Ddl::Phones,
        Flat::Position => Ddl::Position,
        Flat::Rating => Ddl::Rating,
        Flat::RawJson => Ddl::RawJson,
        Flat::Relation => Ddl::Relation,
        Flat::RichText => Ddl::RichText,
        Flat::Select => Ddl::Select,
        Flat::Text => Ddl::Text,
        Flat::TsVector => Ddl::TsVector,
        Flat::Uuid => Ddl::Uuid,
    }
}

fn field_column_input(field: &UniversalFlatFieldMetadata) -> FlatFieldColumnInput {
    FlatFieldColumnInput {
        name: field.name.clone(),
        field_type: ddl_field_type(field.field_type),
        is_nullable: Some(field.is_nullable),
        default_value: field.default_value.clone(),
        universal_identifier: field.universal_identifier.clone(),
        morph_id: field.morph_id.map(|id| id.to_string()),
        relation_type: relation_type_from_settings(&field.universal_settings),
    }
}

fn relation_type_from_settings(settings: &Option<Value>) -> Option<String> {
    settings
        .as_ref()
        .and_then(|value| value.get("relationType"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn foreign_key_on_delete_from_settings(settings: &Option<Value>) -> Option<String> {
    match settings
        .as_ref()
        .and_then(|value| value.get("onDelete"))
        .and_then(Value::as_str)
    {
        Some("CASCADE") | None => Some("CASCADE".to_string()),
        Some("SET_NULL") => Some("SET NULL".to_string()),
        Some("RESTRICT") => Some("RESTRICT".to_string()),
        Some("NO_ACTION") => Some("NO ACTION".to_string()),
        Some(other) => Some(other.to_string()),
    }
}

fn enum_values_from_field(field: &UniversalFlatFieldMetadata) -> Vec<String> {
    let Some(options) = &field.options else {
        return Vec::new();
    };

    options
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(|value| match value {
                    Value::String(raw) => Some(raw.clone()),
                    Value::Object(object) => object
                        .get("value")
                        .and_then(Value::as_str)
                        .map(String::from),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn is_enum_field_type(field_type: FlatFieldMetadataType) -> bool {
    matches!(
        field_type,
        FlatFieldMetadataType::Select
            | FlatFieldMetadataType::MultiSelect
            | FlatFieldMetadataType::Rating
    )
}

async fn create_select_enum(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    table_name: &str,
    field: &UniversalFlatFieldMetadata,
) -> Result<(), RunnerError> {
    let values = enum_values_from_field(field);

    if values.is_empty() {
        return Ok(());
    }

    let enum_name = compute_postgres_enum_name(table_name, &field.name);
    let value_refs: Vec<&str> = values.iter().map(String::as_str).collect();

    enum_types::create_enum(connection, &runtime.schema_name, &enum_name, &value_refs).await?;

    Ok(())
}

async fn enum_type_exists(
    connection: &mut PgConnection,
    schema_name: &str,
    enum_name: &str,
) -> Result<bool, RunnerError> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_type t \
         JOIN pg_namespace n ON n.oid = t.typnamespace \
         WHERE n.nspname = $1 AND t.typname = $2 AND t.typtype = 'e')",
    )
    .bind(schema_name)
    .bind(enum_name)
    .fetch_one(&mut *connection)
    .await?;

    Ok(row.0)
}

async fn rename_enum_type_if_exists(
    connection: &mut PgConnection,
    schema_name: &str,
    old_enum_name: &str,
    new_enum_name: &str,
) -> Result<(), RunnerError> {
    if old_enum_name == new_enum_name
        || !enum_type_exists(connection, schema_name, old_enum_name).await?
    {
        return Ok(());
    }

    enum_types::rename_enum(connection, schema_name, old_enum_name, new_enum_name).await?;

    Ok(())
}

async fn insert_object_row(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    object: &UniversalFlatObjectMetadata,
    object_id: Uuid,
    label_identifier_field_id: Option<Uuid>,
    image_identifier_field_id: Option<Uuid>,
) -> Result<(), RunnerError> {
    let universal_identifier = if object.universal_identifier.is_empty() {
        object_id
    } else {
        universal_uuid(&object.universal_identifier)?
    };

    let target_table_name = DEPRECATED_TARGET_TABLE_NAME;

    sqlx::query(
        "INSERT INTO core.\"objectMetadata\" \
         (id, \"workspaceId\", \"applicationId\", \"universalIdentifier\", \
          \"nameSingular\", \"namePlural\", \"labelSingular\", \"labelPlural\", \
          description, icon, color, \"targetTableName\", \"isRemote\", \"isActive\", \
          \"isSystem\", \"isSearchable\", \"isAuditLogged\", \"isLabelSyncedWithName\", \
          \"isUIEditable\", \"isUICreatable\", \"duplicateCriteria\", shortcut, \
          \"labelIdentifierFieldMetadataId\", \"imageIdentifierFieldMetadataId\", \
          overrides, \"openRecordIn\", \"disabledObjectPermissionActions\") \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, \
                 $17, $18, $19, $20, $21, $22, $23, $24, $25, \
                 $26::core.\"objectMetadata_openrecordin_enum\", $27)",
    )
    .bind(object_id)
    .bind(runtime.workspace_id)
    .bind(runtime.application_id)
    .bind(universal_identifier)
    .bind(&object.name_singular)
    .bind(&object.name_plural)
    .bind(&object.label_singular)
    .bind(&object.label_plural)
    .bind(&object.description)
    .bind(&object.icon)
    .bind(&object.color)
    .bind(target_table_name)
    .bind(object.is_remote)
    .bind(object.is_active)
    .bind(object.is_system)
    .bind(object.is_searchable)
    .bind(object.is_audit_logged)
    .bind(object.is_label_synced_with_name)
    .bind(object.is_ui_editable)
    .bind(object.is_ui_creatable)
    .bind(&object.duplicate_criteria)
    .bind(&object.shortcut)
    .bind(label_identifier_field_id)
    .bind(image_identifier_field_id)
    .bind(&object.overrides)
    .bind(object.open_record_in.map(|value| value.as_str()))
    .bind(&object.disabled_object_permission_actions)
    .execute(&mut *connection)
    .await?;

    Ok(())
}

async fn insert_field_row(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    field: &UniversalFlatFieldMetadata,
    field_id: Uuid,
    object_id: Uuid,
    preallocated: &PreallocatedIds,
) -> Result<(), RunnerError> {
    let universal_identifier = if field.universal_identifier.is_empty() {
        field_id
    } else {
        universal_uuid(&field.universal_identifier)?
    };

    let relation_target_field_id = match &field.relation_target_field_metadata_universal_identifier
    {
        Some(identifier) if !identifier.is_empty() => {
            match preallocated.get(MetadataName::FieldMetadata, identifier) {
                Some(id) => Some(id),
                None => lookup_field_row(connection, runtime, identifier)
                    .await?
                    .map(|row| row.id),
            }
        }
        _ => None,
    };

    let relation_target_object_id =
        match &field.relation_target_object_metadata_universal_identifier {
            Some(identifier) if !identifier.is_empty() => {
                Some(resolve_object_id(connection, runtime, identifier, preallocated).await?)
            }
            _ => None,
        };

    sqlx::query(
        "INSERT INTO core.\"fieldMetadata\" \
         (id, \"objectMetadataId\", type, name, label, \"defaultValue\", description, icon, \
          options, settings, \"isActive\", \"isSystem\", \"isUIReadOnly\", \"isNullable\", \
          \"workspaceId\", \"isLabelSyncedWithName\", \"relationTargetFieldMetadataId\", \
          \"relationTargetObjectMetadataId\", \"morphId\", \"universalIdentifier\", \
          \"applicationId\", \"isUIEditable\", \"isSystemSideEffect\", overrides, \
          writability, \"isAuditLogged\", \"disabledFieldPermissionActions\") \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, false, $13, $14, $15, \
                 $16, $17, $18, $19, $20, $21, $22, $23, 'OPEN', $24, $25)",
    )
    .bind(field_id)
    .bind(object_id)
    .bind(field.field_type.as_str())
    .bind(&field.name)
    .bind(&field.label)
    .bind(&field.default_value)
    .bind(&field.description)
    .bind(&field.icon)
    .bind(&field.options)
    .bind(&field.universal_settings)
    .bind(field.is_active)
    .bind(field.is_system)
    .bind(field.is_nullable)
    .bind(runtime.workspace_id)
    .bind(field.is_label_synced_with_name)
    .bind(relation_target_field_id)
    .bind(relation_target_object_id)
    .bind(field.morph_id)
    .bind(universal_identifier)
    .bind(runtime.application_id)
    .bind(field.is_ui_editable)
    .bind(field.is_system_side_effect)
    .bind(&field.overrides)
    .bind(field.is_audit_logged)
    .bind(&field.disabled_field_permission_actions)
    .execute(&mut *connection)
    .await?;

    Ok(())
}

pub fn base_column_definitions_for_rebuild() -> Vec<WorkspaceSchemaColumnDefinition> {
    base_column_definitions()
}

fn base_column_definitions() -> Vec<WorkspaceSchemaColumnDefinition> {
    let column = |name: &str, column_type: &str, nullable: bool, default: Option<&str>| {
        WorkspaceSchemaColumnDefinition {
            name: name.to_string(),
            column_type: column_type.to_string(),
            is_nullable: Some(nullable),
            default: default.map(String::from),
            is_primary: false,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        }
    };

    let mut id_column = column("id", "uuid", false, Some("public.uuid_generate_v4()"));
    id_column.is_primary = true;

    vec![
        id_column,
        column(
            "createdAt",
            "timestamp with time zone",
            false,
            Some("now()"),
        ),
        column(
            "updatedAt",
            "timestamp with time zone",
            false,
            Some("now()"),
        ),
        column("deletedAt", "timestamp with time zone", true, None),
    ]
}

const ACCESS_GROUP_IDS_COLUMN_NAME: &str = "_accessGroupIds";
const IS_PRIVATE_COLUMN_NAME: &str = "_isPrivate";

const BASE_COLUMN_NAMES: [&str; 4] = ["id", "createdAt", "updatedAt", "deletedAt"];

fn object_id_for_action(
    create: &UniversalCreateAction,
    metadata_name: MetadataName,
    preallocated: &PreallocatedIds,
    fallback: Uuid,
) -> Result<Uuid, RunnerError> {
    if let Some(raw) = &create.id {
        return universal_uuid(raw);
    }

    if let Some(entity) = &create.flat_entity {
        if let Some(id) = preallocated.get(metadata_name, entity.universal_identifier()) {
            return Ok(id);
        }
    }

    Ok(fallback)
}

fn field_id_for_action(
    create: &UniversalCreateAction,
    field: &UniversalFlatFieldMetadata,
    preallocated: &PreallocatedIds,
    fallback: Uuid,
) -> Result<Uuid, RunnerError> {
    if let Some(raw) = &create.id {
        return universal_uuid(raw);
    }

    if let Some(id) = preallocated.get(MetadataName::FieldMetadata, &field.universal_identifier) {
        return Ok(id);
    }

    Ok(fallback)
}

fn is_base_column_field(field: &UniversalFlatFieldMetadata) -> bool {
    BASE_COLUMN_NAMES.contains(&field.name.as_str())
        && matches!(
            field.field_type,
            FlatFieldMetadataType::Uuid | FlatFieldMetadataType::DateTime
        )
}

async fn create_composite_enum_columns(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    table_name: &str,
    field: &UniversalFlatFieldMetadata,
) -> Result<(), RunnerError> {
    if !composite_types::is_composite_field_metadata_type(ddl_field_type(field.field_type)) {
        return Ok(());
    }

    for property in composite_types::composite_properties(ddl_field_type(field.field_type)) {
        if !composite_property_is_enum(property.field_type) {
            continue;
        }

        if property.options.is_empty() {
            continue;
        }

        let column_name = compute_composite_column_name(&field.name, property.name);
        let enum_name = compute_postgres_enum_name(table_name, &column_name);
        let values: Vec<&str> = property.options.to_vec();

        enum_types::create_enum(connection, &runtime.schema_name, &enum_name, &values).await?;
    }

    Ok(())
}

async fn create_permission_enrichment(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    table_name: &str,
    object_id: Uuid,
) -> Result<(), RunnerError> {
    let hex_id = object_id.simple().to_string();

    let access_group_index = WorkspaceSchemaIndexDefinition {
        name: format!("IDX_ACCESS_GROUP_IDS_{hex_id}"),
        columns: vec![ACCESS_GROUP_IDS_COLUMN_NAME.to_string()],
        is_unique: false,
        index_type: Some("GIN".to_string()),
        where_clause: None,
    };

    index::create_index(
        connection,
        &runtime.schema_name,
        table_name,
        &access_group_index,
    )
    .await?;

    let private_record_index = WorkspaceSchemaIndexDefinition {
        name: format!("IDX_PRIVATE_RECORD_{hex_id}"),
        columns: vec!["id".to_string()],
        is_unique: false,
        index_type: None,
        where_clause: Some("\"_isPrivate\"".to_string()),
    };

    index::create_index(
        connection,
        &runtime.schema_name,
        table_name,
        &private_record_index,
    )
    .await?;

    Ok(())
}

async fn apply_object_create(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    create: &UniversalCreateAction,
    preallocated: &PreallocatedIds,
) -> Result<(), RunnerError> {
    let object = object_entity(create)?.clone();

    let object_id = object_id_for_action(create, MetadataName::ObjectMetadata, preallocated, {
        Uuid::new_v4()
    })?;

    let mut field_ids: std::collections::HashMap<String, Uuid> = create
        .field_id_by_universal_identifier
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(identifier, raw_id)| Some((identifier, Uuid::parse_str(&raw_id).ok()?)))
        .collect();

    for field in &create.universal_flat_field_metadatas {
        if !field_ids.contains_key(&field.universal_identifier) {
            let id = preallocated
                .get(MetadataName::FieldMetadata, &field.universal_identifier)
                .unwrap_or_else(Uuid::new_v4);

            field_ids.insert(field.universal_identifier.clone(), id);
        }
    }

    let table_name = compute_object_target_table(
        &object.name_singular,
        &runtime.application_universal_identifier,
    );

    let label_identifier_field_id = object
        .label_identifier_field_metadata_universal_identifier
        .as_ref()
        .and_then(|identifier| field_ids.get(identifier).copied());

    let image_identifier_field_id = object
        .image_identifier_field_metadata_universal_identifier
        .as_ref()
        .and_then(|identifier| field_ids.get(identifier).copied());

    insert_object_row(
        connection,
        runtime,
        &object,
        object_id,
        label_identifier_field_id,
        image_identifier_field_id,
    )
    .await?;

    for field in &create.universal_flat_field_metadatas {
        let field_id = field_ids[&field.universal_identifier];

        insert_field_row(
            connection,
            runtime,
            field,
            field_id,
            object_id,
            preallocated,
        )
        .await?;

        if is_enum_field_type(field.field_type) {
            create_select_enum(connection, runtime, &table_name, field).await?;
        }

        create_composite_enum_columns(connection, runtime, &table_name, field).await?;
    }

    let mut column_definitions = base_column_definitions();
    let mut column_names: std::collections::HashSet<String> = column_definitions
        .iter()
        .map(|definition| definition.name.clone())
        .collect();

    for field in &create.universal_flat_field_metadatas {
        let search_vector_expression = if field.field_type == FlatFieldMetadataType::TsVector {
            Some(search_vector_expression(connection, runtime, object_id).await?)
        } else {
            None
        };

        let definitions = generate_column_definitions(
            &field_column_input(field),
            &runtime.schema_name,
            &table_name,
            search_vector_expression.as_deref(),
        )
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

        for definition in definitions {
            if column_names.insert(definition.name.clone()) {
                column_definitions.push(definition);
            }
        }
    }

    column_definitions.push(WorkspaceSchemaColumnDefinition {
        name: ACCESS_GROUP_IDS_COLUMN_NAME.to_string(),
        column_type: "integer".to_string(),
        is_nullable: Some(true),
        default: None,
        is_primary: false,
        is_array: true,
        as_expression: None,
        generated_type: None,
        is_generated_always_as_identity: false,
    });

    column_definitions.push(WorkspaceSchemaColumnDefinition {
        name: IS_PRIVATE_COLUMN_NAME.to_string(),
        column_type: "boolean".to_string(),
        is_nullable: Some(false),
        default: Some("false".to_string()),
        is_primary: false,
        is_array: false,
        as_expression: None,
        generated_type: None,
        is_generated_always_as_identity: false,
    });

    if let Some(column_plan) = &create.column_plan {
        let position_by_name: std::collections::HashMap<&str, usize> = column_plan
            .iter()
            .enumerate()
            .map(|(index, name)| (name.as_str(), index))
            .collect();

        column_definitions.sort_by(|left, right| {
            position_by_name
                .get(left.name.as_str())
                .copied()
                .unwrap_or(usize::MAX)
                .cmp(
                    &position_by_name
                        .get(right.name.as_str())
                        .copied()
                        .unwrap_or(usize::MAX),
                )
        });
    }

    table::create_table(
        connection,
        &runtime.schema_name,
        &table_name,
        &column_definitions,
    )
    .await?;

    create_permission_enrichment(connection, runtime, &table_name, object_id).await?;

    Ok(())
}

pub async fn object_enum_column_names(
    connection: &mut PgConnection,
    object_id: Uuid,
) -> Result<Vec<String>, RunnerError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT name, type FROM core.\"fieldMetadata\" WHERE \"objectMetadataId\" = $1",
    )
    .bind(object_id)
    .fetch_all(&mut *connection)
    .await?;

    let mut columns: Vec<String> = Vec::new();

    for (name, raw_type) in rows {
        let Some(field_type) = FlatFieldMetadataType::parse(&raw_type) else {
            continue;
        };

        if is_enum_field_type(field_type) {
            if !columns.contains(&name) {
                columns.push(name);
            }
            continue;
        }

        let ddl_type = ddl_field_type(field_type);

        if composite_types::is_composite_field_metadata_type(ddl_type) {
            for property in composite_types::composite_properties(ddl_type) {
                if !matches!(
                    property.field_type,
                    super::column_defs::FieldMetadataType::Select
                        | super::column_defs::FieldMetadataType::MultiSelect
                        | super::column_defs::FieldMetadataType::Rating
                ) {
                    continue;
                }

                let column = compute_composite_column_name(&name, property.name);

                if !columns.contains(&column) {
                    columns.push(column);
                }
            }
        }
    }

    Ok(columns)
}

pub async fn rename_object_enum_types(
    connection: &mut PgConnection,
    schema_name: &str,
    old_table_name: &str,
    new_table_name: &str,
    column_names: &[String],
) -> Result<(), RunnerError> {
    for column in column_names {
        let old_enum = compute_postgres_enum_name(old_table_name, column);
        let new_enum = compute_postgres_enum_name(new_table_name, column);

        rename_enum_type_if_exists(connection, schema_name, &old_enum, &new_enum).await?;
    }

    Ok(())
}

async fn apply_object_update(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    update: &UniversalUpdateAction,
) -> Result<(), RunnerError> {
    let object_id = lookup_object_id(connection, runtime, &update.universal_identifier).await?;

    let (previous_name_singular, object_application_universal_identifier): (String, String) =
        sqlx::query_as(
            "SELECT om.\"nameSingular\", a.\"universalIdentifier\"::text \
             FROM core.\"objectMetadata\" om \
             JOIN core.\"application\" a ON a.id = om.\"applicationId\" \
             WHERE om.id = $1",
        )
        .bind(object_id)
        .fetch_one(&mut *connection)
        .await?;

    let previous_table = compute_object_target_table(
        &previous_name_singular,
        &object_application_universal_identifier,
    );

    let label_identifier_universal = update
        .update
        .get("labelIdentifierFieldMetadataUniversalIdentifier")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let label_identifier_field_id = match label_identifier_universal {
        Some(identifier) => Some(
            resolve_field_id(connection, runtime, identifier, &PreallocatedIds::default()).await?,
        ),
        None => None,
    };

    let image_identifier_universal = update
        .update
        .get("imageIdentifierFieldMetadataUniversalIdentifier")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let image_identifier_field_id = match image_identifier_universal {
        Some(identifier) => Some(
            resolve_field_id(connection, runtime, identifier, &PreallocatedIds::default()).await?,
        ),
        None => None,
    };

    sqlx::query(
        "UPDATE core.\"objectMetadata\" SET \
         \"nameSingular\" = COALESCE($2, \"nameSingular\"), \
         \"namePlural\" = COALESCE($3, \"namePlural\"), \
         \"labelSingular\" = COALESCE($4, \"labelSingular\"), \
         \"labelPlural\" = COALESCE($5, \"labelPlural\"), \
         description = COALESCE($6, description), \
         icon = COALESCE($7, icon), \
         color = COALESCE($8, color), \
         \"isActive\" = COALESCE($9, \"isActive\"), \
         \"isSearchable\" = COALESCE($10, \"isSearchable\"), \
         \"labelIdentifierFieldMetadataId\" = COALESCE($11, \"labelIdentifierFieldMetadataId\"), \
         \"imageIdentifierFieldMetadataId\" = COALESCE($12, \"imageIdentifierFieldMetadataId\"), \
         \"updatedAt\" = now() \
         WHERE id = $1",
    )
    .bind(object_id)
    .bind(update.update.get("nameSingular").and_then(Value::as_str))
    .bind(update.update.get("namePlural").and_then(Value::as_str))
    .bind(update.update.get("labelSingular").and_then(Value::as_str))
    .bind(update.update.get("labelPlural").and_then(Value::as_str))
    .bind(update.update.get("description").and_then(Value::as_str))
    .bind(update.update.get("icon").and_then(Value::as_str))
    .bind(update.update.get("color").and_then(Value::as_str))
    .bind(update.update.get("isActive").and_then(Value::as_bool))
    .bind(update.update.get("isSearchable").and_then(Value::as_bool))
    .bind(label_identifier_field_id)
    .bind(image_identifier_field_id)
    .execute(&mut *connection)
    .await?;

    if let Some(new_name) = update.update.get("nameSingular").and_then(Value::as_str) {
        let new_table =
            compute_object_target_table(new_name, &object_application_universal_identifier);

        if new_table != previous_table {
            table::rename_table(
                connection,
                &runtime.schema_name,
                &previous_table,
                &new_table,
            )
            .await?;

            let enum_columns = object_enum_column_names(connection, object_id).await?;

            rename_object_enum_types(
                connection,
                &runtime.schema_name,
                &previous_table,
                &new_table,
                &enum_columns,
            )
            .await?;
        }
    }

    Ok(())
}

async fn apply_object_delete(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    delete: &UniversalDeleteAction,
) -> Result<(), RunnerError> {
    let object_id = lookup_object_id(connection, runtime, &delete.universal_identifier).await?;

    let table_name = object_table_name(connection, object_id).await?;
    let enum_columns = object_enum_column_names(connection, object_id).await?;

    sqlx::query("DELETE FROM core.\"objectMetadata\" WHERE id = $1")
        .bind(object_id)
        .execute(&mut *connection)
        .await?;

    table::drop_table(connection, &runtime.schema_name, &table_name, true).await?;

    for column in &enum_columns {
        let enum_name = compute_postgres_enum_name(&table_name, column);
        enum_types::drop_enum(connection, &runtime.schema_name, &enum_name).await?;
    }

    Ok(())
}

async fn apply_field_create(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    create: &UniversalCreateAction,
    should_defer_actions: bool,
    preallocated: &PreallocatedIds,
) -> Result<Option<DeferredOperationInput>, RunnerError> {
    let field = field_entity(create.flat_entity.as_ref())?.clone();

    if field.object_metadata_universal_identifier.is_empty() {
        return Err(RunnerError::execution_failed(
            "field action is missing its object metadata identifier",
        ));
    }

    let object_id = resolve_object_id(
        connection,
        runtime,
        &field.object_metadata_universal_identifier,
        preallocated,
    )
    .await?;

    let field_id = field_id_for_action(create, &field, preallocated, Uuid::new_v4())?;

    insert_field_row(
        connection,
        runtime,
        &field,
        field_id,
        object_id,
        preallocated,
    )
    .await?;

    let table_name = object_table_name(connection, object_id).await?;

    let input = field_column_input(&field);

    if column_defs::is_morph_relation_head(&input) {
        return Ok(None);
    }

    if is_enum_field_type(field.field_type) {
        create_select_enum(connection, runtime, &table_name, &field).await?;
    }

    create_composite_enum_columns(connection, runtime, &table_name, &field).await?;

    let search_vector_expression = if field.field_type == FlatFieldMetadataType::TsVector {
        Some(search_vector_expression(connection, runtime, object_id).await?)
    } else {
        None
    };

    let definitions = generate_column_definitions(
        &input,
        &runtime.schema_name,
        &table_name,
        search_vector_expression.as_deref(),
    )
    .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let many_to_one_target = if matches!(
        field.field_type,
        FlatFieldMetadataType::Relation | FlatFieldMetadataType::MorphRelation
    ) && relation_type_from_settings(&field.universal_settings)
        .as_deref()
        == Some("MANY_TO_ONE")
    {
        field
            .relation_target_object_metadata_universal_identifier
            .as_deref()
            .filter(|identifier| !identifier.is_empty())
    } else {
        None
    };

    if !definitions.is_empty() && !is_base_column_field(&field) {
        column::add_columns(
            connection,
            &runtime.schema_name,
            &table_name,
            &definitions,
            true,
        )
        .await?;
    }

    let mut deferred = None;

    if let Some(target_identifier) = many_to_one_target {
        let join_column = compute_relation_join_column_name(&field.name);
        let target_object_id =
            resolve_object_id(connection, runtime, target_identifier, preallocated).await?;
        let target_table = object_table_name(connection, target_object_id).await?;

        let foreign_key = WorkspaceSchemaForeignKeyDefinition {
            table_name: table_name.clone(),
            column_name: join_column,
            referenced_table_name: target_table,
            referenced_column_name: "id".to_string(),
            on_delete: foreign_key_on_delete_from_settings(&field.universal_settings),
            on_update: None,
        };

        foreign_key::create_foreign_key(connection, &runtime.schema_name, &foreign_key, true)
            .await?;

        if should_defer_actions {
            deferred = Some(DeferredOperationInput {
                name: deferred::VALIDATE_FOREIGN_KEY.to_string(),
                payload: serde_json::json!({ "fieldMetadataId": field_id }),
                application_universal_identifier: runtime.application_universal_identifier.clone(),
            });
        } else {
            foreign_key::validate_foreign_key(connection, &runtime.schema_name, &foreign_key)
                .await?;
        }
    }

    Ok(deferred)
}

async fn apply_field_update(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    update: &UniversalUpdateAction,
) -> Result<(), RunnerError> {
    let existing = lookup_field_row(connection, runtime, &update.universal_identifier)
        .await?
        .ok_or_else(|| {
            RunnerError::execution_failed(format!(
                "field metadata {} not found",
                update.universal_identifier
            ))
        })?;

    let is_morph_head = is_morph_relation_head_row(&existing);

    sqlx::query(
        "UPDATE core.\"fieldMetadata\" SET \
         name = COALESCE($2, name), \
         label = COALESCE($3, label), \
         \"defaultValue\" = COALESCE($4, \"defaultValue\"), \
         \"isNullable\" = COALESCE($5, \"isNullable\"), \
         \"isActive\" = COALESCE($6, \"isActive\"), \
         description = COALESCE($7, description), \
         options = COALESCE($8, options), \
         icon = COALESCE($9, icon), \
         settings = COALESCE($10, settings), \
         \"relationTargetObjectMetadataId\" = COALESCE($11, \"relationTargetObjectMetadataId\"), \
         \"relationTargetFieldMetadataId\" = COALESCE($12, \"relationTargetFieldMetadataId\"), \
         \"updatedAt\" = now() \
         WHERE id = $1",
    )
    .bind(existing.id)
    .bind(update.update.get("name").and_then(Value::as_str))
    .bind(update.update.get("label").and_then(Value::as_str))
    .bind(update.update.get("defaultValue"))
    .bind(update.update.get("isNullable").and_then(Value::as_bool))
    .bind(update.update.get("isActive").and_then(Value::as_bool))
    .bind(update.update.get("description").and_then(Value::as_str))
    .bind(update.update.get("options"))
    .bind(update.update.get("icon").and_then(Value::as_str))
    .bind(update.update.get("universalSettings"))
    .bind(
        update
            .update
            .get("relationTargetObjectMetadataUniversalIdentifier")
            .and_then(Value::as_str)
            .map(universal_uuid)
            .transpose()?,
    )
    .bind(
        update
            .update
            .get("relationTargetFieldMetadataUniversalIdentifier")
            .and_then(Value::as_str)
            .map(universal_uuid)
            .transpose()?,
    )
    .execute(&mut *connection)
    .await?;

    if is_morph_head {
        return Ok(());
    }

    let table_name = object_table_name(connection, existing.object_metadata_id).await?;
    let ddl_type = ddl_field_type(existing.field_type);
    let is_composite = composite_types::is_composite_field_metadata_type(ddl_type);
    let is_relation = is_morph_or_relation_field_type(existing.field_type);

    let optimistic_name = update
        .update
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&existing.name)
        .to_string();

    if let Some(new_name) = update.update.get("name").and_then(Value::as_str) {
        if new_name != existing.name {
            if is_composite {
                for property in composite_types::composite_properties(ddl_type) {
                    let from_column = compute_composite_column_name(&existing.name, property.name);
                    let to_column = compute_composite_column_name(new_name, property.name);
                    column::rename_column(
                        connection,
                        &runtime.schema_name,
                        &table_name,
                        &from_column,
                        &to_column,
                    )
                    .await?;
                }
            } else if is_relation {
                if relation_type_from_settings(&existing.settings).as_deref() == Some("MANY_TO_ONE")
                {
                    column::rename_column(
                        connection,
                        &runtime.schema_name,
                        &table_name,
                        &compute_relation_join_column_name(&existing.name),
                        &compute_relation_join_column_name(new_name),
                    )
                    .await?;
                }
            } else {
                column::rename_column(
                    connection,
                    &runtime.schema_name,
                    &table_name,
                    &existing.name,
                    new_name,
                )
                .await?;
            }

            let enum_columns = enum_columns_for_rename(
                &existing.name,
                new_name,
                ddl_type,
                is_enum_field_type(existing.field_type),
            );

            for (old_column, new_column) in enum_columns {
                let old_enum = compute_postgres_enum_name(&table_name, &old_column);
                let new_enum = compute_postgres_enum_name(&table_name, &new_column);
                rename_enum_type_if_exists(connection, &runtime.schema_name, &old_enum, &new_enum)
                    .await?;
            }
        }
    }

    let optimistic_default = match update.update.get("defaultValue") {
        Some(value) if !value.is_null() => Some(value.clone()),
        Some(_) => Some(Value::Null),
        None => existing.default_value.clone(),
    };

    if update.update.get("defaultValue").is_some() {
        apply_field_default_update(
            connection,
            runtime,
            &table_name,
            &optimistic_name,
            ddl_type,
            optimistic_default.as_ref(),
        )
        .await?;
    }

    if update.update.get("isNullable").is_some() {
        let to_is_nullable = update
            .update
            .get("isNullable")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        apply_field_nullable_update(
            connection,
            runtime,
            &table_name,
            &optimistic_name,
            ddl_type,
            to_is_nullable,
            optimistic_default.as_ref(),
        )
        .await?;
    }

    if update.update.get("universalSettings").is_some() && is_relation {
        let from_on_delete = foreign_key_on_delete_from_settings(&existing.settings);
        let to_settings = update.update.get("universalSettings");
        let to_on_delete = foreign_key_on_delete_from_settings(&to_settings.cloned());

        let is_many_to_one =
            relation_type_from_settings(&existing.settings).as_deref() == Some("MANY_TO_ONE");

        if is_many_to_one && to_on_delete != from_on_delete {
            let join_column = compute_relation_join_column_name(&optimistic_name);

            let foreign_key_name = foreign_key::get_foreign_key_name(
                connection,
                &runtime.schema_name,
                &table_name,
                &join_column,
            )
            .await?
            .ok_or_else(|| {
                RunnerError::execution_failed(format!(
                    "NOT_SUPPORTED: Foreign key not found on {table_name}.{join_column}"
                ))
            })?;

            let target_object_id =
                existing.relation_target_object_metadata_id.ok_or_else(|| {
                    RunnerError::execution_failed(
                        "OBJECT_METADATA_NOT_FOUND: relation field has no target object metadata",
                    )
                })?;

            let target_table = object_table_name(connection, target_object_id).await?;

            foreign_key::drop_foreign_key(
                connection,
                &runtime.schema_name,
                &table_name,
                &foreign_key_name,
            )
            .await?;

            let foreign_key = WorkspaceSchemaForeignKeyDefinition {
                table_name: table_name.clone(),
                column_name: join_column,
                referenced_table_name: target_table,
                referenced_column_name: "id".to_string(),
                on_delete: to_on_delete,
                on_update: None,
            };

            foreign_key::create_foreign_key(connection, &runtime.schema_name, &foreign_key, false)
                .await?;
        }
    }

    if update.rebuild_search_vector == Some(true)
        && existing.field_type == FlatFieldMetadataType::TsVector
    {
        let expression =
            search_vector_expression(connection, runtime, existing.object_metadata_id).await?;

        column::drop_columns(
            connection,
            &runtime.schema_name,
            &table_name,
            &[optimistic_name.as_str()],
            false,
        )
        .await?;

        let definitions = vec![WorkspaceSchemaColumnDefinition {
            name: optimistic_name.clone(),
            column_type: "tsvector".to_string(),
            is_nullable: Some(true),
            default: None,
            is_primary: false,
            is_array: false,
            as_expression: Some(expression),
            generated_type: Some(GeneratedType::Stored),
            is_generated_always_as_identity: false,
        }];

        column::add_columns(
            connection,
            &runtime.schema_name,
            &table_name,
            &definitions,
            false,
        )
        .await?;

        let related_indexes: Vec<(String, bool, Option<String>, String)> = sqlx::query_as(
            "SELECT im.name, im.\"isUnique\", im.\"indexWhereClause\", im.\"indexType\"::text \
             FROM core.\"indexMetadata\" im \
             JOIN core.\"indexFieldMetadata\" ifm ON ifm.\"indexMetadataId\" = im.id \
             WHERE ifm.\"fieldMetadataId\" = $1",
        )
        .bind(existing.id)
        .fetch_all(&mut *connection)
        .await?;

        for (index_name, is_unique, where_clause, index_type) in related_indexes {
            let definition = WorkspaceSchemaIndexDefinition {
                name: index_name,
                columns: vec![optimistic_name.clone()],
                is_unique,
                index_type: Some(index_type),
                where_clause,
            };

            index::create_index(connection, &runtime.schema_name, &table_name, &definition).await?;
        }
    }

    Ok(())
}

fn composite_property_is_enum(field_type: super::column_defs::FieldMetadataType) -> bool {
    matches!(
        field_type,
        super::column_defs::FieldMetadataType::Select
            | super::column_defs::FieldMetadataType::MultiSelect
            | super::column_defs::FieldMetadataType::Rating
    )
}

fn enum_columns_for_rename(
    old_name: &str,
    new_name: &str,
    ddl_type: super::column_defs::FieldMetadataType,
    is_enum_field: bool,
) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    if is_enum_field {
        pairs.push((old_name.to_string(), new_name.to_string()));
    }

    if composite_types::is_composite_field_metadata_type(ddl_type) {
        for property in composite_types::composite_properties(ddl_type) {
            if !composite_property_is_enum(property.field_type) {
                continue;
            }

            pairs.push((
                compute_composite_column_name(old_name, property.name),
                compute_composite_column_name(new_name, property.name),
            ));
        }
    }

    pairs
}

async fn apply_field_default_update(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    table_name: &str,
    field_name: &str,
    ddl_type: super::column_defs::FieldMetadataType,
    default_value: Option<&Value>,
) -> Result<(), RunnerError> {
    if ddl_type == super::column_defs::FieldMetadataType::TsVector {
        return Ok(());
    }

    if composite_types::is_composite_field_metadata_type(ddl_type) {
        for property in composite_types::composite_properties(ddl_type) {
            let column_name = compute_composite_column_name(field_name, property.name);
            let column_type = column_defs::field_metadata_type_to_column_type(property.field_type)
                .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

            let property_default = default_value
                .filter(|value| value.is_object())
                .and_then(|value| value.get(property.name));

            let serialized = column_defs::serialize_default_value(
                property_default,
                column_type,
                &runtime.schema_name,
                table_name,
                &column_name,
            )
            .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

            let alteration = if serialized == "NULL" {
                column::AlterColumnDefault::SetNull
            } else {
                column::AlterColumnDefault::Literal(serialized)
            };

            column::alter_column_default(
                connection,
                &runtime.schema_name,
                table_name,
                &column_name,
                alteration,
            )
            .await?;
        }

        return Ok(());
    }

    let column_type = column_defs::field_metadata_type_to_column_type(ddl_type)
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let serialized = column_defs::serialize_default_value(
        default_value,
        column_type,
        &runtime.schema_name,
        table_name,
        field_name,
    )
    .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let alteration = if serialized == "NULL" {
        column::AlterColumnDefault::SetNull
    } else {
        column::AlterColumnDefault::Literal(serialized)
    };

    column::alter_column_default(
        connection,
        &runtime.schema_name,
        table_name,
        field_name,
        alteration,
    )
    .await
    .map_err(|error| RunnerError::execution_failed(error.to_string()))
}

async fn apply_field_nullable_update(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    table_name: &str,
    field_name: &str,
    ddl_type: super::column_defs::FieldMetadataType,
    to_is_nullable: bool,
    default_value: Option<&Value>,
) -> Result<(), RunnerError> {
    if matches!(
        ddl_type,
        super::column_defs::FieldMetadataType::Relation
            | super::column_defs::FieldMetadataType::MorphRelation
    ) || ddl_type == super::column_defs::FieldMetadataType::TsVector
    {
        return Ok(());
    }

    if composite_types::is_composite_field_metadata_type(ddl_type) {
        for property in composite_types::composite_properties(ddl_type) {
            let column_name = compute_composite_column_name(field_name, property.name);

            column::alter_column_nullable(
                connection,
                &runtime.schema_name,
                table_name,
                &column_name,
                true,
                None,
            )
            .await?;
        }

        return Ok(());
    }

    let backfill_value = if to_is_nullable {
        None
    } else {
        let column_type = column_defs::field_metadata_type_to_column_type(ddl_type)
            .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

        let serialized = column_defs::serialize_default_value(
            default_value,
            column_type,
            &runtime.schema_name,
            table_name,
            field_name,
        )
        .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

        if serialized == "NULL" {
            None
        } else {
            Some(serialized)
        }
    };

    column::alter_column_nullable(
        connection,
        &runtime.schema_name,
        table_name,
        field_name,
        to_is_nullable,
        backfill_value.as_deref(),
    )
    .await
    .map_err(|error| RunnerError::execution_failed(error.to_string()))
}

async fn apply_field_delete(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    delete: &UniversalDeleteAction,
) -> Result<(), RunnerError> {
    let Some(existing) =
        lookup_field_row(connection, runtime, &delete.universal_identifier).await?
    else {
        return Ok(());
    };

    sqlx::query("DELETE FROM core.\"fieldMetadata\" WHERE id = $1")
        .bind(existing.id)
        .execute(&mut *connection)
        .await?;

    if is_morph_relation_head_row(&existing) {
        return Ok(());
    }

    let table_name = object_table_name(connection, existing.object_metadata_id).await?;
    let ddl_type = ddl_field_type(existing.field_type);

    let column_names = if composite_types::is_composite_field_metadata_type(ddl_type) {
        composite_types::composite_properties(ddl_type)
            .iter()
            .map(|property| compute_composite_column_name(&existing.name, property.name))
            .collect()
    } else if matches!(
        existing.field_type,
        FlatFieldMetadataType::Relation | FlatFieldMetadataType::MorphRelation
    ) {
        vec![compute_relation_join_column_name(&existing.name)]
    } else {
        vec![existing.name.clone()]
    };

    let column_refs: Vec<&str> = column_names.iter().map(String::as_str).collect();

    column::drop_columns(
        connection,
        &runtime.schema_name,
        &table_name,
        &column_refs,
        true,
    )
    .await?;

    if is_enum_field_type(existing.field_type) {
        let enum_name = compute_postgres_enum_name(&table_name, &existing.name);
        enum_types::drop_enum(connection, &runtime.schema_name, &enum_name).await?;
    }

    if composite_types::is_composite_field_metadata_type(ddl_type) {
        for property in composite_types::composite_properties(ddl_type) {
            if !composite_property_is_enum(property.field_type) {
                continue;
            }

            let column_name = compute_composite_column_name(&existing.name, property.name);
            let enum_name = compute_postgres_enum_name(&table_name, &column_name);

            enum_types::drop_enum(connection, &runtime.schema_name, &enum_name).await?;
        }
    }

    Ok(())
}

pub async fn index_column_names(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    index: &UniversalFlatIndexMetadata,
) -> Result<Vec<String>, RunnerError> {
    let mut columns = Vec::new();

    let mut ordered = index.universal_flat_index_field_metadatas.clone();
    ordered.sort_by_key(|field| field.order);

    for index_field in ordered {
        let Some(field) = lookup_field_row(
            connection,
            runtime,
            &index_field.field_metadata_universal_identifier,
        )
        .await?
        else {
            continue;
        };

        if matches!(
            field.field_type,
            FlatFieldMetadataType::Relation | FlatFieldMetadataType::MorphRelation
        ) {
            columns.push(compute_relation_join_column_name(&field.name));
        } else if let Some(sub_field_name) = &index_field.sub_field_name {
            columns.push(compute_composite_column_name(&field.name, sub_field_name));
        } else if composite_types::is_composite_field_metadata_type(ddl_field_type(
            field.field_type,
        )) {
            let properties: &[CompositeProperty] =
                composite_types::composite_properties(ddl_field_type(field.field_type));

            for property in properties
                .iter()
                .filter(|property| property.is_included_in_unique_constraint)
            {
                columns.push(compute_composite_column_name(&field.name, property.name));
            }
        } else {
            columns.push(field.name.clone());
        }
    }

    Ok(columns)
}

pub fn index_is_deferrable(index: &UniversalFlatIndexMetadata, column_count: usize) -> bool {
    !index.is_unique
        && index.index_where_clause.is_none()
        && column_count > 0
        && index.index_type == IndexType::Btree
}

pub async fn index_definition(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    index: &UniversalFlatIndexMetadata,
) -> Result<WorkspaceSchemaIndexDefinition, RunnerError> {
    let columns = index_column_names(connection, runtime, index).await?;

    Ok(WorkspaceSchemaIndexDefinition {
        name: index.name.clone(),
        columns,
        is_unique: index.is_unique,
        index_type: Some(index.index_type.as_str().to_string()),
        where_clause: index.index_where_clause.clone(),
    })
}

async fn insert_index_row(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    index: &UniversalFlatIndexMetadata,
    preallocated: &PreallocatedIds,
) -> Result<Uuid, RunnerError> {
    let index_id = Uuid::new_v4();

    let object_id = resolve_object_id(
        connection,
        runtime,
        &index.object_metadata_universal_identifier,
        preallocated,
    )
    .await?;

    sqlx::query(
        "INSERT INTO core.\"indexMetadata\" \
         (id, name, \"workspaceId\", \"objectMetadataId\", \"isUnique\", \
          \"indexWhereClause\", \"indexType\", \"universalIdentifier\", \"applicationId\", \
          \"isSystemSideEffect\") \
         VALUES ($1, $2, $3, $4, $5, $6, $7::core.\"indexMetadata_indextype_enum\", $8, $9, $10)",
    )
    .bind(index_id)
    .bind(&index.name)
    .bind(runtime.workspace_id)
    .bind(object_id)
    .bind(index.is_unique)
    .bind(&index.index_where_clause)
    .bind(index.index_type.as_str())
    .bind(universal_uuid(&index.universal_identifier)?)
    .bind(runtime.application_id)
    .bind(index.is_system_side_effect)
    .execute(&mut *connection)
    .await?;

    for index_field in &index.universal_flat_index_field_metadatas {
        let field_id = resolve_field_id(
            connection,
            runtime,
            &index_field.field_metadata_universal_identifier,
            preallocated,
        )
        .await?;

        if let Some(sub_field_name) = &index_field.sub_field_name {
            sqlx::query(
                "INSERT INTO core.\"indexFieldMetadata\" \
                 (id, \"indexMetadataId\", \"fieldMetadataId\", \"workspaceId\", \"order\", \
                  \"subFieldName\") \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(Uuid::new_v4())
            .bind(index_id)
            .bind(field_id)
            .bind(runtime.workspace_id)
            .bind(index_field.order)
            .bind(sub_field_name)
            .execute(&mut *connection)
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO core.\"indexFieldMetadata\" \
                 (id, \"indexMetadataId\", \"fieldMetadataId\", \"workspaceId\", \"order\") \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(Uuid::new_v4())
            .bind(index_id)
            .bind(field_id)
            .bind(runtime.workspace_id)
            .bind(index_field.order)
            .execute(&mut *connection)
            .await?;
        }
    }

    Ok(index_id)
}

async fn apply_index_create(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    create: &UniversalCreateAction,
    should_defer_actions: bool,
    preallocated: &PreallocatedIds,
) -> Result<Option<DeferredOperationInput>, RunnerError> {
    let index = index_entity(create.flat_entity.as_ref())?.clone();

    let index_id = insert_index_row(connection, runtime, &index, preallocated).await?;

    let definition = index_definition(connection, runtime, &index).await?;

    if should_defer_actions && index_is_deferrable(&index, definition.columns.len()) {
        return Ok(Some(DeferredOperationInput {
            name: deferred::BUILD_INDEX.to_string(),
            payload: serde_json::json!({ "indexMetadataId": index_id }),
            application_universal_identifier: runtime.application_universal_identifier.clone(),
        }));
    }

    let validated =
        index::validate_and_return_index_where_clause(definition.where_clause.as_deref())?;

    let definition = WorkspaceSchemaIndexDefinition {
        where_clause: validated,
        ..definition
    };

    let object_id = lookup_object_id(
        connection,
        runtime,
        &index.object_metadata_universal_identifier,
    )
    .await?;
    let table_name = object_table_name(connection, object_id).await?;

    index::create_index(connection, &runtime.schema_name, &table_name, &definition).await?;

    Ok(None)
}

async fn apply_index_update(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    update: &UniversalUpdateAction,
) -> Result<(), RunnerError> {
    let Some(updated_index) = update.updated_universal_flat_index.clone() else {
        return Ok(());
    };

    let existing: Option<(Uuid, String)> = sqlx::query_as(
        "SELECT id, name FROM core.\"indexMetadata\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(runtime.workspace_id)
    .bind(universal_uuid(&update.universal_identifier)?)
    .fetch_optional(&mut *connection)
    .await?;

    let Some((index_id, previous_name)) = existing else {
        return Err(RunnerError::execution_failed(format!(
            "FLAT_ENTITY_NOT_FOUND: index metadata {} not found",
            update.universal_identifier
        )));
    };

    let object_id = lookup_object_id(
        connection,
        runtime,
        &updated_index.object_metadata_universal_identifier,
    )
    .await?;
    let table_name = object_table_name(connection, object_id).await?;

    index::drop_index(connection, &runtime.schema_name, &previous_name, false).await?;

    sqlx::query("DELETE FROM core.\"indexMetadata\" WHERE id = $1")
        .bind(index_id)
        .execute(&mut *connection)
        .await?;

    insert_index_row(
        connection,
        runtime,
        &updated_index,
        &PreallocatedIds::default(),
    )
    .await?;

    let definition = index_definition(connection, runtime, &updated_index).await?;

    let validated =
        index::validate_and_return_index_where_clause(definition.where_clause.as_deref())?;

    let definition = WorkspaceSchemaIndexDefinition {
        where_clause: validated,
        ..definition
    };

    index::create_index(connection, &runtime.schema_name, &table_name, &definition).await?;

    Ok(())
}

async fn apply_index_delete(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    delete: &UniversalDeleteAction,
) -> Result<(), RunnerError> {
    let existing: Option<(Uuid, String, Uuid)> = sqlx::query_as(
        "SELECT id, name, \"objectMetadataId\" FROM core.\"indexMetadata\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(runtime.workspace_id)
    .bind(universal_uuid(&delete.universal_identifier)?)
    .fetch_optional(&mut *connection)
    .await?;

    let Some((index_id, name, _object_id)) = existing else {
        return Ok(());
    };

    index::drop_index(connection, &runtime.schema_name, &name, false).await?;

    sqlx::query("DELETE FROM core.\"indexMetadata\" WHERE id = $1")
        .bind(index_id)
        .execute(&mut *connection)
        .await?;

    Ok(())
}

pub struct OptionRow {
    pub universal_identifier: String,
    pub value: String,
    pub position: f64,
}

async fn option_rows_for_field(
    connection: &mut PgConnection,
    field_id: Uuid,
) -> Result<Vec<OptionRow>, RunnerError> {
    let rows: Vec<(String, String, f64)> = sqlx::query_as(
        "SELECT \"universalIdentifier\"::text, value, \"position\" \
         FROM core.\"fieldMetadataOption\" WHERE \"fieldMetadataId\" = $1",
    )
    .bind(field_id)
    .fetch_all(&mut *connection)
    .await?;

    let mut options: Vec<OptionRow> = rows
        .into_iter()
        .map(|(universal_identifier, value, position)| OptionRow {
            universal_identifier,
            value,
            position,
        })
        .collect();

    options.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.universal_identifier.cmp(&b.universal_identifier))
    });

    Ok(options)
}

async fn current_enum_values(
    connection: &mut PgConnection,
    schema_name: &str,
    enum_name: &str,
) -> Result<Vec<String>, RunnerError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT e.enumlabel::text FROM pg_type t \
         JOIN pg_namespace n ON n.oid = t.typnamespace \
         JOIN pg_enum e ON e.enumtypid = t.oid \
         WHERE n.nspname = $1 AND t.typname = $2 AND t.typtype = 'e' \
         ORDER BY e.enumsortorder ASC",
    )
    .bind(schema_name)
    .bind(enum_name)
    .fetch_all(&mut *connection)
    .await?;

    Ok(rows.into_iter().map(|(label,)| label).collect())
}

pub async fn apply_enum_change_from_option_rows(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    field: &FieldRow,
    table_name: &str,
    from_rows: &[OptionRow],
    to_rows: &[OptionRow],
) -> Result<bool, RunnerError> {
    if to_rows.is_empty() {
        return Ok(false);
    }

    let to_value_by_identifier: std::collections::HashMap<&str, &str> = to_rows
        .iter()
        .map(|row| (row.universal_identifier.as_str(), row.value.as_str()))
        .collect();

    let mut to_values: Vec<String> = Vec::new();
    for row in to_rows {
        if !to_values.iter().any(|value| value == &row.value) {
            to_values.push(row.value.clone());
        }
    }

    let mut old_to_new_map: Vec<(String, String)> = Vec::new();
    for row in from_rows {
        let mapped = to_value_by_identifier
            .get(row.universal_identifier.as_str())
            .map(|value| value.to_string())
            .or_else(|| {
                to_values
                    .iter()
                    .any(|value| value == &row.value)
                    .then(|| row.value.clone())
            });

        if let Some(to_value) = mapped {
            if !old_to_new_map
                .iter()
                .any(|(from_value, _)| from_value == &row.value)
            {
                old_to_new_map.push((row.value.clone(), to_value));
            }
        }
    }

    let is_value_moved = old_to_new_map
        .iter()
        .any(|(from_value, to_value)| from_value != to_value);

    let from_values: Vec<&str> = from_rows.iter().map(|row| row.value.as_str()).collect();

    let enum_name = compute_postgres_enum_name(table_name, &field.name);
    let current_values = current_enum_values(connection, &runtime.schema_name, &enum_name).await?;

    let mut enum_values = to_values.clone();
    for value in &current_values {
        if !from_values.contains(&value.as_str()) && !enum_values.iter().any(|v| v == value) {
            enum_values.push(value.clone());
        }
    }

    if !is_value_moved
        && enum_values.len() == current_values.len()
        && enum_values
            .iter()
            .zip(current_values.iter())
            .all(|(a, b)| a == b)
    {
        return Ok(false);
    }

    for value in enum_values
        .iter()
        .filter(|value| {
            !old_to_new_map
                .iter()
                .any(|(from_value, _)| from_value == *value)
                && !current_values.contains(value)
        })
        .cloned()
        .collect::<Vec<String>>()
    {
        old_to_new_map.push((value.clone(), value.clone()));
    }

    let column_type = if enum_name.is_empty() {
        String::new()
    } else {
        format!("\"{}\".\"{}\"", runtime.schema_name, enum_name)
    };

    let is_array = field.field_type == FlatFieldMetadataType::MultiSelect;

    let serialized_default = column_defs::serialize_default_value(
        field.default_value.as_ref(),
        "enum",
        &runtime.schema_name,
        table_name,
        &field.name,
    )
    .map_err(|error| RunnerError::execution_failed(error.to_string()))?;

    let column_definition = WorkspaceSchemaColumnDefinition {
        name: field.name.clone(),
        column_type,
        is_nullable: Some(field.is_nullable),
        default: Some(serialized_default),
        is_primary: false,
        is_array,
        as_expression: None,
        generated_type: None,
        is_generated_always_as_identity: false,
    };

    let value_refs: Vec<&str> = enum_values.iter().map(String::as_str).collect();

    enum_types::alter_enum_values(
        connection,
        &runtime.schema_name,
        table_name,
        &column_definition,
        &value_refs,
        &old_to_new_map,
    )
    .await?;

    Ok(true)
}

async fn sync_field_enum_from_option_rows(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    field_id: Uuid,
    from_rows: &[OptionRow],
) -> Result<(), RunnerError> {
    let field: Option<(Uuid, String, String, String, bool, Option<Value>)> = sqlx::query_as(
        "SELECT id, \"objectMetadataId\"::text, name, type, COALESCE(\"isNullable\", true), \
         \"defaultValue\" \
         FROM core.\"fieldMetadata\" WHERE id = $1",
    )
    .bind(field_id)
    .fetch_optional(&mut *connection)
    .await?;

    let Some((id, object_id_raw, name, raw_type, is_nullable, default_value)) = field else {
        return Ok(());
    };

    let Some(field_type) = FlatFieldMetadataType::parse(&raw_type) else {
        return Ok(());
    };

    if !is_enum_field_type(field_type) {
        return Ok(());
    }

    let object_id = universal_uuid(&object_id_raw)?;
    let table_name = object_table_name(connection, object_id).await?;
    let to_rows = option_rows_for_field(connection, id).await?;

    let field_row = FieldRow {
        id,
        object_metadata_id: object_id,
        label: name.clone(),
        name,
        field_type,
        is_nullable,
        default_value,
        settings: None,
        relation_target_object_metadata_id: None,
        universal_identifier: String::new(),
        morph_id: None,
    };

    apply_enum_change_from_option_rows(
        connection,
        runtime,
        &field_row,
        &table_name,
        from_rows,
        &to_rows,
    )
    .await?;

    Ok(())
}

async fn apply_option_create(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    create: &UniversalCreateAction,
) -> Result<(), RunnerError> {
    let option = option_entity(create.flat_entity.as_ref())?.clone();

    let field = lookup_field_row(
        connection,
        runtime,
        &option.field_metadata_universal_identifier,
    )
    .await?
    .ok_or_else(|| RunnerError::execution_failed("option field metadata not found"))?;

    let from_rows = option_rows_for_field(connection, field.id).await?;

    sqlx::query(
        "INSERT INTO core.\"fieldMetadataOption\" \
         (id, \"workspaceId\", \"universalIdentifier\", \"applicationId\", overrides, \
          \"isActive\", \"fieldMetadataId\", value, label, color, \"position\") \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(Uuid::new_v4())
    .bind(runtime.workspace_id)
    .bind(universal_uuid(&option.universal_identifier)?)
    .bind(runtime.application_id)
    .bind(&option.overrides)
    .bind(option.is_active)
    .bind(field.id)
    .bind(&option.value)
    .bind(&option.label)
    .bind(&option.color)
    .bind(option.position)
    .execute(&mut *connection)
    .await?;

    sync_field_enum_from_option_rows(connection, runtime, field.id, &from_rows).await?;

    Ok(())
}

async fn apply_option_update(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    update: &UniversalUpdateAction,
) -> Result<(), RunnerError> {
    let existing: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, \"fieldMetadataId\" FROM core.\"fieldMetadataOption\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(runtime.workspace_id)
    .bind(universal_uuid(&update.universal_identifier)?)
    .fetch_optional(&mut *connection)
    .await?;

    let Some((option_id, field_id)) = existing else {
        return Ok(());
    };

    let from_rows = option_rows_for_field(connection, field_id).await?;

    sqlx::query(
        "UPDATE core.\"fieldMetadataOption\" SET \
         value = COALESCE($3, value), \
         label = COALESCE($4, label), \
         color = COALESCE($5, color), \
         \"position\" = COALESCE($6, \"position\"), \
         \"isActive\" = COALESCE($7, \"isActive\"), \
         \"updatedAt\" = now() \
         WHERE id = $1",
    )
    .bind(option_id)
    .bind(update.update.get("value").and_then(Value::as_str))
    .bind(update.update.get("label").and_then(Value::as_str))
    .bind(update.update.get("color").and_then(Value::as_str))
    .bind(update.update.get("position").and_then(Value::as_f64))
    .bind(update.update.get("isActive").and_then(Value::as_bool))
    .execute(&mut *connection)
    .await?;

    sync_field_enum_from_option_rows(connection, runtime, field_id, &from_rows).await?;

    Ok(())
}

async fn apply_option_delete(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    delete: &UniversalDeleteAction,
) -> Result<(), RunnerError> {
    let existing: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, \"fieldMetadataId\" FROM core.\"fieldMetadataOption\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(runtime.workspace_id)
    .bind(universal_uuid(&delete.universal_identifier)?)
    .fetch_optional(&mut *connection)
    .await?;

    let Some((option_id, field_id)) = existing else {
        return Ok(());
    };

    let from_rows = option_rows_for_field(connection, field_id).await?;

    sqlx::query("DELETE FROM core.\"fieldMetadataOption\" WHERE id = $1")
        .bind(option_id)
        .execute(&mut *connection)
        .await?;

    sync_field_enum_from_option_rows(connection, runtime, field_id, &from_rows).await?;

    Ok(())
}

async fn apply_search_field_create(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    create: &UniversalCreateAction,
    preallocated: &PreallocatedIds,
) -> Result<(), RunnerError> {
    let search_field = search_field_entity(create.flat_entity.as_ref())?.clone();

    let object_id = resolve_object_id(
        connection,
        runtime,
        &search_field.object_metadata_universal_identifier,
        preallocated,
    )
    .await?;

    let field_id = resolve_field_id(
        connection,
        runtime,
        &search_field.field_metadata_universal_identifier,
        preallocated,
    )
    .await?;

    let ts_vector_field_id = resolve_field_id(
        connection,
        runtime,
        &search_field.ts_vector_field_metadata_universal_identifier,
        preallocated,
    )
    .await?;

    sqlx::query(
        "INSERT INTO core.\"searchFieldMetadata\" \
         (id, \"objectMetadataId\", \"fieldMetadataId\", \"workspaceId\", \
          \"universalIdentifier\", \"applicationId\", \"position\", \"tsVectorFieldMetadataId\", \
          \"isSystemSideEffect\") \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(object_id_for_action(
        create,
        MetadataName::SearchFieldMetadata,
        preallocated,
        Uuid::new_v4(),
    )?)
    .bind(object_id)
    .bind(field_id)
    .bind(runtime.workspace_id)
    .bind(universal_uuid(&search_field.universal_identifier)?)
    .bind(runtime.application_id)
    .bind(search_field.position)
    .bind(ts_vector_field_id)
    .bind(search_field.is_system_side_effect)
    .execute(&mut *connection)
    .await?;

    Ok(())
}

pub async fn resolve_object_id(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    universal_identifier: &str,
    preallocated: &PreallocatedIds,
) -> Result<Uuid, RunnerError> {
    if let Some(id) = preallocated.get(MetadataName::ObjectMetadata, universal_identifier) {
        return Ok(id);
    }

    lookup_object_id(connection, runtime, universal_identifier).await
}

pub async fn resolve_field_id(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    universal_identifier: &str,
    preallocated: &PreallocatedIds,
) -> Result<Uuid, RunnerError> {
    if let Some(id) = preallocated.get(MetadataName::FieldMetadata, universal_identifier) {
        return Ok(id);
    }

    lookup_field_row(connection, runtime, universal_identifier)
        .await?
        .map(|row| row.id)
        .ok_or_else(|| {
            RunnerError::execution_failed(format!(
                "field metadata {universal_identifier} not found"
            ))
        })
}

async fn apply_search_field_update(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    update: &UniversalUpdateAction,
) -> Result<(), RunnerError> {
    sqlx::query(
        "UPDATE core.\"searchFieldMetadata\" SET position = COALESCE($3, position), \
         \"updatedAt\" = now() WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(runtime.workspace_id)
    .bind(universal_uuid(&update.universal_identifier)?)
    .bind(update.update.get("position").and_then(Value::as_f64))
    .execute(&mut *connection)
    .await?;

    Ok(())
}

async fn apply_search_field_delete(
    connection: &mut PgConnection,
    runtime: &WorkspaceRuntime,
    delete: &UniversalDeleteAction,
) -> Result<(), RunnerError> {
    sqlx::query(
        "DELETE FROM core.\"searchFieldMetadata\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(runtime.workspace_id)
    .bind(universal_uuid(&delete.universal_identifier)?)
    .execute(&mut *connection)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_create_action() -> UniversalAction {
        UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::ObjectMetadata,
            flat_entity: Some(UniversalFlatEntity::ObjectMetadata(Box::new(
                UniversalFlatObjectMetadata {
                    universal_identifier: Uuid::new_v4().to_string(),
                    name_singular: "rocket".to_string(),
                    name_plural: "rockets".to_string(),
                    label_singular: "Rocket".to_string(),
                    label_plural: "Rockets".to_string(),
                    is_active: true,
                    ..UniversalFlatObjectMetadata::default()
                },
            ))),
            ..UniversalCreateAction::default()
        })
    }

    #[test]
    fn schema_affecting_names_match_the_upstream_set() {
        assert!(is_schema_affecting(MetadataName::ObjectMetadata));
        assert!(is_schema_affecting(MetadataName::FieldMetadata));
        assert!(is_schema_affecting(MetadataName::FieldMetadataOption));
        assert!(is_schema_affecting(MetadataName::Index));
        assert!(!is_schema_affecting(MetadataName::View));
        assert!(!is_schema_affecting(MetadataName::SearchFieldMetadata));
    }

    #[test]
    fn schema_affecting_migrations_are_detected() {
        let migration = WorkspaceMigration {
            actions: vec![object_create_action()],
            application_universal_identifier: "app".to_string(),
        };

        assert!(migration_is_schema_affecting(&migration));

        let metadata_only = WorkspaceMigration {
            actions: vec![UniversalAction::Delete(UniversalDeleteAction {
                metadata_name: MetadataName::View,
                universal_identifier: Uuid::new_v4().to_string(),
                flat_entity: None,
            })],
            application_universal_identifier: "app".to_string(),
        };

        assert!(!migration_is_schema_affecting(&metadata_only));
    }

    #[test]
    fn base_columns_carry_the_reference_shape() {
        let definitions = base_column_definitions();

        assert_eq!(definitions.len(), 4);
        assert_eq!(definitions[0].name, "id");
        assert!(definitions[0].is_primary);
        assert_eq!(definitions[1].name, "createdAt");
        assert_eq!(definitions[3].name, "deletedAt");
        assert_eq!(definitions[3].is_nullable, Some(true));
    }

    #[test]
    fn deferrable_indexes_are_the_plain_btree_ones() {
        let index = UniversalFlatIndexMetadata {
            name: "IDX_test".to_string(),
            index_type: IndexType::Btree,
            ..UniversalFlatIndexMetadata::default()
        };

        assert!(index_is_deferrable(&index, 1));

        let unique = UniversalFlatIndexMetadata {
            is_unique: true,
            ..index.clone()
        };
        assert!(!index_is_deferrable(&unique, 1));

        let partial = UniversalFlatIndexMetadata {
            index_where_clause: Some("\"deletedAt\" IS NULL".to_string()),
            ..index.clone()
        };
        assert!(!index_is_deferrable(&partial, 1));

        let gin = UniversalFlatIndexMetadata {
            index_type: IndexType::Gin,
            ..index.clone()
        };
        assert!(!index_is_deferrable(&gin, 1));

        assert!(!index_is_deferrable(&index, 0));
    }

    #[test]
    fn action_type_dispatch_matches_the_action_variant() {
        let action = object_create_action();

        assert_eq!(action.action_type(), WorkspaceMigrationActionType::Create);
        assert!(expect_create(&action).is_ok());
        assert!(expect_update(&action).is_err());
        assert!(expect_delete(&action).is_err());
    }

    #[test]
    fn error_codes_match_the_upstream_strings() {
        assert_eq!(RunnerError::ddl_locked().code, "DDL_LOCKED");
        assert_eq!(
            RunnerError::deferred_in_progress().code,
            "DEFERRED_WORKSPACE_MIGRATION_ACTIONS_IN_PROGRESS"
        );
        assert_eq!(
            RunnerError::application_not_found("x").code,
            "APPLICATION_NOT_FOUND"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use nudge_core::NodeEnv;

    const TEST_SCHEMA_PREFIX: &str = "wm_runner";

    static TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn test_config(pg_database_url: String) -> nudge_core::Config {
        nudge_core::Config {
            node_env: NodeEnv::Test,
            node_port: 3999,
            server_url: "http://localhost:3999".to_string(),
            pg_database_url,
            pg_pool_max_connections: 4,
            redis_url: "redis://127.0.0.1:6399".to_string(),
            redis_cache_prefix: String::new(),
            app_secret: "integration-test-secret".to_string(),
            encryption_key: None,
            fallback_encryption_key: None,
            access_token_expires_in_secs: 1800,
            refresh_token_expires_in_secs: 3600,
            login_token_expires_in_secs: 900,
            workspace_agnostic_token_expires_in_secs: 1800,
            refresh_token_reuse_grace_period_secs: 60,
            frontend_url: "http://localhost:3999".to_string(),
            auth_cookie_same_site: "lax".to_string(),
            auth_cookie_allowed_origins: String::new(),
            session_absolute_lifetime_secs: 3600,
            session_idle_timeout_secs: 3600,
            is_cloudflare_geolocation_enabled: false,
            workspace_schema_ddl_locked: false,
        }
    }

    async fn test_state() -> Option<AppState> {
        let url = std::env::var("NUDGE_TEST_DATABASE_URL").ok()?;
        let state = AppState::connect(&test_config(url.clone()))
            .await
            .expect("connecting the test state");

        crate::workspace_migrations::ddl::test_support::migrate_core(&url)
            .await
            .expect("running core migrations");

        Some(state)
    }

    struct TestWorkspace {
        workspace_id: Uuid,
        application_universal_identifier: String,
        schema_name: String,
    }

    async fn create_test_workspace(state: &AppState) -> TestWorkspace {
        let workspace_id = Uuid::new_v4();
        let application_id = Uuid::new_v4();
        let application_universal_identifier = Uuid::new_v4().to_string();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let subdomain = format!("runner-{}", &Uuid::new_v4().simple().to_string()[..10]);

        let default_role_id = Uuid::new_v4();

        let mut transaction = state.core_pool.begin().await.expect("tx");

        sqlx::query(
            "INSERT INTO core.\"workspace\" \
             (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
              \"workspaceCustomApplicationId\", \"isPasswordAuthEnabled\", \"databaseSchema\") \
             VALUES ($1, 'Runner Test', $2, 'PENDING_CREATION', $3, $4, true, $5)",
        )
        .bind(workspace_id)
        .bind(Uuid::new_v4())
        .bind(&subdomain)
        .bind(application_id)
        .bind(&schema_name)
        .execute(&mut *transaction)
        .await
        .expect("inserting the test workspace");

        sqlx::query(
            "INSERT INTO core.\"application\" \
             (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", \"workspaceId\") \
             VALUES ($1, $2, 'Runner Test Application', 'LOCAL', 'INSTALLED', '', $3)",
        )
        .bind(application_id)
        .bind(
            Uuid::parse_str(&application_universal_identifier)
                .expect("application universal identifier is a uuid"),
        )
        .bind(workspace_id)
        .execute(&mut *transaction)
        .await
        .expect("inserting the test application");

        sqlx::query(
            "INSERT INTO core.\"role\" \
             (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\") \
             VALUES ($1, $2, 'Member', $3, $4)",
        )
        .bind(default_role_id)
        .bind(Uuid::new_v4())
        .bind(application_id)
        .bind(workspace_id)
        .execute(&mut *transaction)
        .await
        .expect("inserting the default role");

        sqlx::query(
            "UPDATE core.\"workspace\" SET \"activationStatus\" = 'CREATED', \
                \"defaultRoleId\" = $2 WHERE id = $1",
        )
        .bind(workspace_id)
        .bind(default_role_id)
        .execute(&mut *transaction)
        .await
        .expect("activating the test workspace");

        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name))
            .execute(&mut *transaction)
            .await
            .expect("creating the workspace schema");

        transaction.commit().await.expect("committing the fixture");

        TestWorkspace {
            workspace_id,
            application_universal_identifier,
            schema_name,
        }
    }

    fn object_action(
        name_singular: &str,
        application_universal_identifier: &str,
        field: UniversalFlatFieldMetadata,
    ) -> UniversalAction {
        let object_universal_identifier = Uuid::new_v4().to_string();

        UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::ObjectMetadata,
            id: Some(Uuid::new_v4().to_string()),
            flat_entity: Some(UniversalFlatEntity::ObjectMetadata(Box::new(
                UniversalFlatObjectMetadata {
                    universal_identifier: object_universal_identifier.clone(),
                    application_universal_identifier: application_universal_identifier.to_string(),
                    name_singular: name_singular.to_string(),
                    name_plural: format!("{name_singular}s"),
                    label_singular: name_singular.to_string(),
                    label_plural: format!("{name_singular}s"),
                    is_active: true,
                    target_table_name: Some(format!("_{name_singular}")),
                    ..UniversalFlatObjectMetadata::default()
                },
            ))),
            universal_flat_field_metadatas: vec![field],
            ..UniversalCreateAction::default()
        })
    }

    fn text_field_action(name: &str, object_universal_identifier: &str) -> UniversalAction {
        UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::FieldMetadata,
            flat_entity: Some(UniversalFlatEntity::FieldMetadata(Box::new(
                UniversalFlatFieldMetadata {
                    universal_identifier: Uuid::new_v4().to_string(),
                    object_metadata_universal_identifier: object_universal_identifier.to_string(),
                    field_type: FlatFieldMetadataType::Text,
                    name: name.to_string(),
                    label: name.to_string(),
                    is_active: true,
                    is_nullable: true,
                    ..UniversalFlatFieldMetadata::default()
                },
            ))),
            ..UniversalCreateAction::default()
        })
    }

    fn object_identifier(action: &UniversalAction) -> String {
        match action {
            UniversalAction::Create(create) => match create.flat_entity.as_ref() {
                Some(UniversalFlatEntity::ObjectMetadata(object)) => {
                    object.universal_identifier.clone()
                }
                _ => String::new(),
            },
            _ => String::new(),
        }
    }

    async fn metadata_version(state: &AppState, workspace_id: Uuid) -> i32 {
        let row: (i32,) =
            sqlx::query_as("SELECT \"metadataVersion\" FROM core.\"workspace\" WHERE id = $1")
                .bind(workspace_id)
                .fetch_one(&state.core_pool)
                .await
                .expect("reading metadata version");

        row.0
    }

    async fn table_exists(state: &AppState, schema: &str, table: &str) -> bool {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.tables \
             WHERE table_schema = $1 AND table_name = $2)",
        )
        .bind(schema)
        .bind(table)
        .fetch_one(&state.core_pool)
        .await
        .expect("checking table existence");

        row.0
    }

    async fn column_exists(state: &AppState, schema: &str, table: &str, column: &str) -> bool {
        let row: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
             WHERE table_schema = $1 AND table_name = $2 AND column_name = $3)",
        )
        .bind(schema)
        .bind(table)
        .bind(column)
        .fetch_one(&state.core_pool)
        .await
        .expect("checking column existence");

        row.0
    }

    #[tokio::test]
    async fn run_creates_the_object_row_and_the_workspace_table() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state).await;

        let object = object_action(
            "runnerwidget",
            &workspace.application_universal_identifier,
            UniversalFlatFieldMetadata {
                universal_identifier: Uuid::new_v4().to_string(),
                field_type: FlatFieldMetadataType::Text,
                name: "labelText".to_string(),
                label: "Label".to_string(),
                is_active: true,
                is_nullable: true,
                ..UniversalFlatFieldMetadata::default()
            },
        );

        let migration = WorkspaceMigration {
            actions: vec![object],
            application_universal_identifier: workspace.application_universal_identifier.clone(),
        };

        let before = metadata_version(&state, workspace.workspace_id).await;

        let outcome = run_workspace_migration(
            &state,
            RunnerInput {
                workspace_id: workspace.workspace_id,
                migration: &migration,
                should_defer_actions: true,
            },
        )
        .await
        .expect("running the migration");

        assert_eq!(outcome.metadata_version, Some(before + 1));
        assert_eq!(
            metadata_version(&state, workspace.workspace_id).await,
            before + 1
        );

        assert!(
            table_exists(&state, &workspace.schema_name, "_runnerwidget").await,
            "the workspace table must exist"
        );
        assert!(
            column_exists(&state, &workspace.schema_name, "_runnerwidget", "labelText").await,
            "the field column must exist"
        );

        let object_rows: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM core.\"objectMetadata\" \
             WHERE \"workspaceId\" = $1 AND \"nameSingular\" = 'runnerwidget'",
        )
        .bind(workspace.workspace_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("counting object rows");

        assert_eq!(object_rows.0, 1);

        let field_rows: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM core.\"fieldMetadata\" \
             WHERE \"workspaceId\" = $1 AND name = 'labelText'",
        )
        .bind(workspace.workspace_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("counting field rows");

        assert_eq!(field_rows.0, 1);
    }

    #[tokio::test]
    async fn a_failing_action_rolls_back_the_rows_and_the_ddl() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state).await;

        let object = object_action(
            "rollbackwidget",
            &workspace.application_universal_identifier,
            UniversalFlatFieldMetadata {
                universal_identifier: Uuid::new_v4().to_string(),
                field_type: FlatFieldMetadataType::Text,
                name: "keepText".to_string(),
                label: "Keep".to_string(),
                is_active: true,
                is_nullable: true,
                ..UniversalFlatFieldMetadata::default()
            },
        );

        let failing_field = UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::FieldMetadata,
            flat_entity: Some(UniversalFlatEntity::FieldMetadata(Box::new(
                UniversalFlatFieldMetadata {
                    universal_identifier: Uuid::new_v4().to_string(),
                    object_metadata_universal_identifier: Uuid::new_v4().to_string(),
                    field_type: FlatFieldMetadataType::Text,
                    name: "orphanText".to_string(),
                    label: "Orphan".to_string(),
                    is_active: true,
                    is_nullable: true,
                    ..UniversalFlatFieldMetadata::default()
                },
            ))),
            ..UniversalCreateAction::default()
        });

        let migration = WorkspaceMigration {
            actions: vec![object, failing_field],
            application_universal_identifier: workspace.application_universal_identifier.clone(),
        };

        let before = metadata_version(&state, workspace.workspace_id).await;

        let error = run_workspace_migration(
            &state,
            RunnerInput {
                workspace_id: workspace.workspace_id,
                migration: &migration,
                should_defer_actions: true,
            },
        )
        .await
        .expect_err("the migration must fail");

        assert_eq!(error.code, "EXECUTION_FAILED");
        assert_eq!(error.action_index, Some(1));

        assert!(
            !table_exists(&state, &workspace.schema_name, "_rollbackwidget").await,
            "the table must be rolled back"
        );

        let object_rows: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM core.\"objectMetadata\" \
             WHERE \"workspaceId\" = $1 AND \"nameSingular\" = 'rollbackwidget'",
        )
        .bind(workspace.workspace_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("counting object rows");

        assert_eq!(object_rows.0, 0, "the object row must be rolled back");
        assert_eq!(
            metadata_version(&state, workspace.workspace_id).await,
            before
        );
    }

    #[tokio::test]
    async fn metadata_only_migrations_do_not_bump_the_version() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state).await;

        let migration = WorkspaceMigration {
            actions: vec![UniversalAction::Delete(UniversalDeleteAction {
                metadata_name: MetadataName::View,
                universal_identifier: Uuid::new_v4().to_string(),
                flat_entity: None,
            })],
            application_universal_identifier: workspace.application_universal_identifier.clone(),
        };

        let before = metadata_version(&state, workspace.workspace_id).await;

        let error = run_workspace_migration(
            &state,
            RunnerInput {
                workspace_id: workspace.workspace_id,
                migration: &migration,
                should_defer_actions: true,
            },
        )
        .await
        .expect_err("a view action has no runner handler yet");

        assert_eq!(error.code, "UNSUPPORTED_METADATA_NAME");
        assert_eq!(
            metadata_version(&state, workspace.workspace_id).await,
            before
        );
    }

    #[tokio::test]
    async fn a_deferrable_index_persists_and_executes() {
        let _guard = TEST_LOCK.lock().await;

        let Some(state) = test_state().await else {
            return;
        };

        let workspace = create_test_workspace(&state).await;

        let field_universal_identifier = Uuid::new_v4().to_string();

        let object = object_action(
            "indexwidget",
            &workspace.application_universal_identifier,
            UniversalFlatFieldMetadata {
                universal_identifier: field_universal_identifier.clone(),
                field_type: FlatFieldMetadataType::Text,
                name: "codeText".to_string(),
                label: "Code".to_string(),
                is_active: true,
                is_nullable: true,
                ..UniversalFlatFieldMetadata::default()
            },
        );

        let object_universal_identifier = object_identifier(&object);

        let migration = WorkspaceMigration {
            actions: vec![object],
            application_universal_identifier: workspace.application_universal_identifier.clone(),
        };

        run_workspace_migration(
            &state,
            RunnerInput {
                workspace_id: workspace.workspace_id,
                migration: &migration,
                should_defer_actions: false,
            },
        )
        .await
        .expect("creating the object and field");

        let index_action = UniversalAction::Create(UniversalCreateAction {
            metadata_name: MetadataName::Index,
            flat_entity: Some(UniversalFlatEntity::IndexMetadata(Box::new(
                UniversalFlatIndexMetadata {
                    universal_identifier: Uuid::new_v4().to_string(),
                    object_metadata_universal_identifier: object_universal_identifier.clone(),
                    name: "IDX_runner_code".to_string(),
                    index_type: IndexType::Btree,
                    universal_flat_index_field_metadatas: vec![
                        crate::metadata::flat::UniversalFlatIndexFieldMetadata {
                            order: 0,
                            field_metadata_universal_identifier: field_universal_identifier.clone(),
                            ..Default::default()
                        },
                    ],
                    ..UniversalFlatIndexMetadata::default()
                },
            ))),
            ..UniversalCreateAction::default()
        });

        let index_migration = WorkspaceMigration {
            actions: vec![index_action],
            application_universal_identifier: workspace.application_universal_identifier.clone(),
        };

        let outcome = run_workspace_migration(
            &state,
            RunnerInput {
                workspace_id: workspace.workspace_id,
                migration: &index_migration,
                should_defer_actions: true,
            },
        )
        .await
        .expect("running the deferred index migration");

        assert_eq!(outcome.deferred_operations, 1);

        let pending: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM core.\"deferredWorkspaceOperation\" \
             WHERE \"workspaceId\" = $1 AND name = 'build_index' AND status = 'PENDING'",
        )
        .bind(workspace.workspace_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("counting deferred operations");

        assert_eq!(pending.0, 1);

        let executed = deferred::run_pending_inline(&state, workspace.workspace_id)
            .await
            .expect("executing the deferred operations");

        assert_eq!(executed, 1);

        let index_exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM pg_indexes \
             WHERE schemaname = $1 AND indexname = 'IDX_runner_code')",
        )
        .bind(&workspace.schema_name)
        .fetch_one(&state.core_pool)
        .await
        .expect("checking the index");

        assert!(index_exists.0, "the deferred index must be built");

        let remaining: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM core.\"deferredWorkspaceOperation\" \
             WHERE \"workspaceId\" = $1 AND name = 'build_index'",
        )
        .bind(workspace.workspace_id)
        .fetch_one(&state.core_pool)
        .await
        .expect("counting remaining operations");

        assert_eq!(remaining.0, 0, "successful operations must be removed");

        let _ = TEST_SCHEMA_PREFIX;
    }
}
