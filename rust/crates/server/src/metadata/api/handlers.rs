use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use super::errors::GraphQlError;
use super::graphql::FieldSelection;
use super::input::optional_string;
use super::service::build_options;
use super::service::compute_api_from_to_maps;
use super::service::field_dto;
use super::service::flat_field_universal;
use super::service::flat_index_universal;
use super::service::flat_object_universal;
use super::service::index_dto;
use super::service::input_argument;
use super::service::inputs_argument;
use super::service::load_flat_maps;
use super::service::load_universal_maps;
use super::service::object_dto;
use super::service::option_operations_for_update;
use super::service::project;
use super::service::resolve_applications;
use super::service::resolve_target_field;
use super::service::resolve_target_object;
use super::service::translate_object_identifier_fields;
use super::service::transpile_create_field;
use super::service::transpile_create_object;
use super::service::valid_metadata_name_shape;
use super::service::CreateFieldTranspilation;
use super::service::OperationContext;
use super::service::OptionOperation;
use crate::file::FileUrlSigner;
use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalFlatEntity;
use crate::metadata::builders::build_workspace_migration;
use crate::metadata::builders::expand_matrix_with_side_effects;
use crate::metadata::builders::FlatEntityOperationsMatrix;
use crate::metadata::builders::WorkspaceMigrationBuildResult;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::UniversalFlatFieldMetadata;
use crate::metadata::flat::UniversalFlatIndexFieldMetadata;
use crate::metadata::flat::UniversalFlatIndexMetadata;
use crate::metadata::flat::UniversalFlatObjectMetadata;
use crate::permissions::current_user_workspace_permissions_for_auth;
use crate::state::AppState;
use crate::workspace_migrations::runner::run_workspace_migration;
use crate::workspace_migrations::runner::RunnerInput;

pub struct ResponsePayload {
    pub data: Option<Value>,
    pub errors: Vec<GraphQlError>,
}

impl ResponsePayload {
    pub fn data(data: Value) -> Self {
        Self {
            data: Some(data),
            errors: Vec::new(),
        }
    }

    pub fn errors(errors: Vec<GraphQlError>) -> Self {
        Self { data: None, errors }
    }

    pub fn to_json(&self) -> Value {
        let mut payload = Map::new();

        payload.insert("data".to_string(), self.data.clone().unwrap_or(Value::Null));

        if !self.errors.is_empty() {
            payload.insert(
                "errors".to_string(),
                Value::Array(self.errors.iter().map(GraphQlError::to_json).collect()),
            );
        }

        Value::Object(payload)
    }
}

pub async fn dispatch(ctx: &OperationContext<'_>) -> ResponsePayload {
    let operation_name = ctx.selection.name.as_str();

    let result: Result<Value, GraphQlError> = match operation_name {
        "createOneObject" if !ctx.is_query => create_one_object(ctx).await,
        "updateOneObject" if !ctx.is_query => update_one_object(ctx).await,
        "updateManyObjects" if !ctx.is_query => update_many_objects(ctx).await,
        "deleteOneObject" if !ctx.is_query => delete_one_object(ctx).await,
        "createOneField" | "createFieldMetadata" if !ctx.is_query => create_one_field(ctx).await,
        "updateOneField" | "updateFieldMetadata" if !ctx.is_query => update_one_field(ctx).await,
        "deleteOneField" | "deleteFieldMetadata" if !ctx.is_query => delete_one_field(ctx).await,
        "createOneIndex" if !ctx.is_query => create_one_index(ctx).await,
        "deleteOneIndex" if !ctx.is_query => delete_one_index(ctx).await,
        "objects" if ctx.is_query => list_objects(ctx).await,
        "fields" if ctx.is_query => list_fields(ctx).await,
        "object" if ctx.is_query => get_object(ctx).await,
        "field" if ctx.is_query => get_field(ctx).await,
        "currentUser" if ctx.is_query => current_user(ctx).await,
        "addQueryToEventStream" if !ctx.is_query => add_query_to_event_stream(ctx).await,
        "removeQueryFromEventStream" if !ctx.is_query => remove_query_from_event_stream(ctx).await,
        _ => Err(GraphQlError::validation(format!(
            "Cannot query field \"{operation_name}\" on type."
        ))),
    };

    match result {
        Ok(node) => {
            let projected = project(&node, &ctx.selection.selection);

            ResponsePayload::data(json!({ ctx.selection.response_key(): projected }))
        }
        Err(error) => ResponsePayload::errors(vec![error]),
    }
}

async fn add_query_to_event_stream(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);
    let input = input
        .as_object()
        .ok_or_else(|| GraphQlError::user_input("input is required"))?;
    let event_stream_id = optional_string(input, "eventStreamId")?
        .ok_or_else(|| GraphQlError::user_input("eventStreamId is required"))?;
    let query_id = optional_string(input, "queryId")?
        .ok_or_else(|| GraphQlError::user_input("queryId is required"))?;
    let operation_signature = input
        .get("operationSignature")
        .cloned()
        .ok_or_else(|| GraphQlError::user_input("operationSignature is required"))?;
    let operation_signature = crate::events::RecordOperationSignature::parse(operation_signature)
        .map_err(GraphQlError::user_input)?;
    let auth = ctx
        .auth
        .ok_or_else(|| GraphQlError::unauthenticated("Authentication required"))?;
    Ok(Value::Bool(ctx.state.event_streams.add_query(
        ctx.workspace_id,
        &event_stream_id,
        auth,
        query_id,
        operation_signature,
    )))
}

async fn remove_query_from_event_stream(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);
    let input = input
        .as_object()
        .ok_or_else(|| GraphQlError::user_input("input is required"))?;
    let event_stream_id = optional_string(input, "eventStreamId")?
        .ok_or_else(|| GraphQlError::user_input("eventStreamId is required"))?;
    let query_id = optional_string(input, "queryId")?
        .ok_or_else(|| GraphQlError::user_input("queryId is required"))?;
    let auth = ctx
        .auth
        .ok_or_else(|| GraphQlError::unauthenticated("Authentication required"))?;
    Ok(Value::Bool(ctx.state.event_streams.remove_query(
        ctx.workspace_id,
        &event_stream_id,
        auth,
        &query_id,
    )))
}

async fn run_migration(
    ctx: &OperationContext<'_>,
    matrix: FlatEntityOperationsMatrix,
    application_universal_identifier: &str,
    failure_message: &str,
) -> Result<(), GraphQlError> {
    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    let universal_maps = load_universal_maps(ctx.state, ctx.workspace_id).await?;

    let expansion = expand_matrix_with_side_effects(matrix, &universal_maps);

    let infer_deletion = super::service::infer_deletion_for_deletes(&expansion.matrix);
    let from_to = compute_api_from_to_maps(&universal_maps, &expansion.matrix);

    let options = build_options(
        &applications,
        application_universal_identifier,
        infer_deletion,
    );

    let migration = match build_workspace_migration(options, from_to) {
        WorkspaceMigrationBuildResult::Success(migration) => migration,
        WorkspaceMigrationBuildResult::Fail(report) => {
            return Err(GraphQlError::builder_failure(failure_message, &report))
        }
    };

    let mut migration = migration;

    super::service::normalize_create_action_entities(&mut migration.actions);

    run_workspace_migration(
        ctx.state,
        RunnerInput {
            workspace_id: ctx.workspace_id,
            migration: &migration,
            should_defer_actions: false,
        },
    )
    .await
    .map_err(|error| GraphQlError::runner(&error))?;

    Ok(())
}

pub fn add_to_create(matrix: &mut FlatEntityOperationsMatrix, entity: UniversalFlatEntity) {
    matrix
        .bucket_mut(entity.metadata_name())
        .to_create
        .push(entity);
}

pub fn add_to_update(matrix: &mut FlatEntityOperationsMatrix, entity: UniversalFlatEntity) {
    matrix
        .bucket_mut(entity.metadata_name())
        .to_update
        .push(entity);
}

pub fn add_to_delete(matrix: &mut FlatEntityOperationsMatrix, entity: UniversalFlatEntity) {
    let bucket = matrix.bucket_mut(entity.metadata_name());

    let identifier = entity.universal_identifier().to_string();

    let already_present = bucket
        .to_delete
        .iter()
        .any(|existing| existing.universal_identifier() == identifier);

    if !already_present {
        bucket.to_delete.push(entity);
    }
}

fn object_entity(object: &UniversalFlatObjectMetadata) -> UniversalFlatEntity {
    UniversalFlatEntity::ObjectMetadata(Box::new(object.clone()))
}

fn field_entity(field: &UniversalFlatFieldMetadata) -> UniversalFlatEntity {
    UniversalFlatEntity::FieldMetadata(Box::new(field.clone()))
}

async fn create_one_object(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);

    super::service::validate_create_object_input(&input)?;

    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    let transpilation = transpile_create_object(&input, &applications)?;

    let mut matrix = FlatEntityOperationsMatrix::default();

    add_to_create(&mut matrix, object_entity(&transpilation.entity));

    if let Some(name_field) = &transpilation.name_field {
        add_to_create(&mut matrix, field_entity(name_field));
    }

    run_migration(
        ctx,
        matrix,
        &applications.custom_universal_identifier,
        "Multiple validation errors occurred while creating object",
    )
    .await?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let created = flat_maps
        .object_metadata_maps
        .find_by_universal_identifier(&transpilation.entity.universal_identifier)
        .ok_or_else(|| {
            GraphQlError::internal("Created object metadata not found in recomputed cache")
        })?;

    Ok(object_dto(created))
}

async fn update_one_object(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let results = update_objects(ctx, vec![input_argument(ctx.selection, ctx.variables)]).await?;

    results
        .into_iter()
        .next()
        .ok_or_else(|| GraphQlError::internal("Could not retrieve updated object metadata dto"))
}

async fn update_many_objects(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let inputs = inputs_argument(ctx.selection, ctx.variables);

    if inputs.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }

    update_objects(ctx, inputs).await.map(Value::Array)
}

async fn update_objects(
    ctx: &OperationContext<'_>,
    inputs: Vec<Value>,
) -> Result<Vec<Value>, GraphQlError> {
    if inputs.len() > super::service::MAX_OBJECTS_PER_BATCH_UPDATE {
        return Err(GraphQlError::user_input(format!(
            "Cannot update more than {} objects in one batch",
            super::service::MAX_OBJECTS_PER_BATCH_UPDATE
        )));
    }

    let mut ids: Vec<String> = Vec::new();

    for input in &inputs {
        let id = require_update_id(input)?;

        if ids.contains(&id) {
            return Err(GraphQlError::user_input(
                "Cannot update the same object twice in one batch",
            ));
        }

        ids.push(id);
    }

    if inputs.len() > 1 {
        for input in &inputs {
            if let Some(update) = input.get("update").and_then(Value::as_object) {
                for property in super::service::NON_BATCHABLE_UPDATE_PROPERTIES {
                    if update.contains_key(*property) {
                        return Err(GraphQlError::user_input(format!(
                            "Cannot update {property} in a batch of several objects, send it one at a time"
                        )));
                    }
                }
            }
        }
    }

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let mut matrix = FlatEntityOperationsMatrix::default();

    for (input, id) in inputs.iter().zip(ids.iter()) {
        if let Some(update) = input.get("update").and_then(Value::as_object) {
            for prop in ["nameSingular", "namePlural"] {
                require_string_shape(update, prop)?;
            }
        }

        let existing =
            resolve_target_object(id, &flat_maps.object_metadata_maps).map_err(|_| {
                GraphQlError::not_found("Object to update not found")
                    .with_extensions(json!({ "subCode": "OBJECT_METADATA_NOT_FOUND" }))
            })?;

        let payload = super::service::normalize_update_object_payload(
            input.get("update").unwrap_or(&Value::Null),
        );

        let payload = translate_object_identifier_fields(payload, &existing, &flat_maps)?;

        let merged =
            super::service::merged_object_entity(&flat_object_universal(&existing), &payload);

        add_to_update(&mut matrix, object_entity(&merged));
    }

    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    let message = if ids.len() > 1 {
        "Multiple validation errors occurred while updating objects"
    } else {
        "Multiple validation errors occurred while updating object"
    };

    run_migration(
        ctx,
        matrix,
        &applications.custom_universal_identifier,
        message,
    )
    .await?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let mut results: Vec<Value> = Vec::new();

    for id in &ids {
        let Some(updated) = flat_maps.object_metadata_maps.find_by_id(id) else {
            return Err(GraphQlError::internal(
                "Object metadata disappeared after update",
            ));
        };

        results.push(object_dto(updated));
    }

    Ok(results)
}

fn require_string_shape(object: &Map<String, Value>, prop: &str) -> Result<(), GraphQlError> {
    if let Some(value) = object.get(prop).and_then(Value::as_str) {
        if !valid_metadata_name_shape(value) {
            return Err(GraphQlError::user_input(format!(
                "{prop} has failed the name validation check"
            )));
        }
    }

    Ok(())
}

fn require_update_id(input: &Value) -> Result<String, GraphQlError> {
    let object = input
        .as_object()
        .ok_or_else(|| GraphQlError::validation("Object inputs must be objects"))?;

    let id = super::input::require_string(object, "id")?;

    require_uuid_like(&id)
}

pub fn require_uuid_like(value: &str) -> Result<String, GraphQlError> {
    Uuid::parse_str(value)
        .map(|parsed| parsed.to_string())
        .map_err(|_| GraphQlError::validation("id must be a valid UUID"))
}

async fn delete_one_object(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);

    let id = require_update_id(&input)?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let existing = resolve_target_object(&id, &flat_maps.object_metadata_maps).map_err(|_| {
        GraphQlError::not_found("Object to delete not found")
            .with_extensions(json!({ "subCode": "OBJECT_METADATA_NOT_FOUND" }))
    })?;

    let mut matrix = FlatEntityOperationsMatrix::default();

    add_to_delete(
        &mut matrix,
        object_entity(&flat_object_universal(&existing)),
    );

    for field in flat_maps
        .field_metadata_maps
        .by_universal_identifier
        .values()
    {
        if field.object_metadata_id == existing.id
            || field
                .relation_target_object_metadata_universal_identifier
                .as_deref()
                == Some(existing.universal_identifier.as_str())
        {
            add_to_delete(&mut matrix, field_entity(&flat_field_universal(field)));
        }
    }

    for index in flat_maps
        .index_metadata_maps
        .by_universal_identifier
        .values()
    {
        if index.object_metadata_id == existing.id {
            add_to_delete(
                &mut matrix,
                UniversalFlatEntity::IndexMetadata(Box::new(flat_index_universal(index))),
            );
        }
    }

    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    run_migration(
        ctx,
        matrix,
        &applications.custom_universal_identifier,
        "Multiple validation errors occurred while deleting object",
    )
    .await?;

    Ok(object_dto(&existing))
}

async fn create_one_field(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);

    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    super::service::validate_create_field_input(&input)?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let CreateFieldTranspilation {
        field,
        inverse,
        indexes,
        options,
    } = transpile_create_field(&input, &flat_maps, &applications)?;

    let mut matrix = FlatEntityOperationsMatrix::default();

    add_to_create(&mut matrix, field_entity(&field));

    if let Some(inverse) = &inverse {
        add_to_create(&mut matrix, field_entity(inverse));
    }

    for index in &indexes {
        add_to_create(
            &mut matrix,
            UniversalFlatEntity::IndexMetadata(Box::new(index.clone())),
        );
    }

    if field.is_unique && !field.is_relation_or_morph() {
        let object_universal = flat_object_universal(
            flat_maps
                .object_metadata_maps
                .find_by_universal_identifier(&field.object_metadata_universal_identifier)
                .ok_or_else(|| {
                    GraphQlError::not_found("Provided object metadata id does not exist")
                        .with_extensions(json!({ "subCode": "OBJECT_METADATA_NOT_FOUND" }))
                })?,
        );

        let backing = super::service::build_unique_backing_index(&object_universal, &field);

        add_to_create(
            &mut matrix,
            UniversalFlatEntity::IndexMetadata(Box::new(backing)),
        );
    }

    for option in &options {
        add_to_create(
            &mut matrix,
            UniversalFlatEntity::FieldMetadataOption(Box::new(option.clone())),
        );
    }

    run_migration(
        ctx,
        matrix,
        &applications.custom_universal_identifier,
        "Multiple validation errors occurred while creating field",
    )
    .await?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let created = flat_maps
        .field_metadata_maps
        .find_by_universal_identifier(&field.universal_identifier)
        .ok_or_else(|| GraphQlError::internal("Failed to create field metadata"))?;

    Ok(field_dto(created, &flat_maps))
}

async fn update_one_field(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);

    let id = require_update_id(&input)?;

    let payload =
        super::service::normalize_update_field_payload(input.get("update").unwrap_or(&Value::Null));

    if let Some(object) = payload.as_object() {
        require_string_shape(object, "name")?;
    }

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let existing = resolve_target_field(&id, &flat_maps.field_metadata_maps).map_err(|_| {
        super::errors::field_transpilation_failure(
            "FIELD_METADATA_NOT_FOUND",
            "Field metadata to update not found",
            "update",
            "Multiple validation errors occurred while updating field",
        )
    })?;

    let merged = super::service::merged_field_entity(&flat_field_universal(&existing), &payload);

    let mut matrix = FlatEntityOperationsMatrix::default();

    add_to_update(&mut matrix, field_entity(&merged));

    if payload.get("options").is_some() {
        for operation in option_operations_for_update(&merged, &flat_maps) {
            match operation {
                OptionOperation::Create(option) => {
                    matrix
                        .bucket_mut(MetadataName::FieldMetadataOption)
                        .to_create
                        .push(UniversalFlatEntity::FieldMetadataOption(Box::new(option)));
                }
                OptionOperation::Update(option) => {
                    matrix
                        .bucket_mut(MetadataName::FieldMetadataOption)
                        .to_update
                        .push(UniversalFlatEntity::FieldMetadataOption(Box::new(option)));
                }
                OptionOperation::Delete(option) => {
                    matrix
                        .bucket_mut(MetadataName::FieldMetadataOption)
                        .to_delete
                        .push(UniversalFlatEntity::FieldMetadataOption(Box::new(option)));
                }
            }
        }
    }

    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    if payload.get("isUnique").and_then(Value::as_bool) == Some(true)
        && !merged.is_relation_or_morph()
    {
        let already_backed = flat_maps
            .index_metadata_maps
            .by_universal_identifier
            .values()
            .any(|index| {
                index.is_system_side_effect
                    && index.is_unique
                    && backs_exactly(index, &merged.universal_identifier)
            });

        if !already_backed {
            let Some(object) = flat_maps
                .object_metadata_maps
                .find_by_universal_identifier(&merged.object_metadata_universal_identifier)
            else {
                return Err(GraphQlError::internal("Object metadata not found"));
            };

            let backing =
                super::service::build_unique_backing_index(&flat_object_universal(object), &merged);

            add_to_create(
                &mut matrix,
                UniversalFlatEntity::IndexMetadata(Box::new(backing)),
            );
        }
    }

    if payload.get("isUnique").and_then(Value::as_bool) == Some(false) {
        let stale: Vec<crate::metadata::flat::FlatIndexMetadata> = flat_maps
            .index_metadata_maps
            .by_universal_identifier
            .values()
            .filter(|index| {
                index.is_system_side_effect
                    && index.is_unique
                    && backs_exactly(index, &merged.universal_identifier)
            })
            .cloned()
            .collect();

        for index in stale {
            add_to_delete(
                &mut matrix,
                UniversalFlatEntity::IndexMetadata(Box::new(flat_index_universal(&index))),
            );
        }
    }

    run_migration(
        ctx,
        matrix,
        &applications.custom_universal_identifier,
        "Multiple validation errors occurred while updating field",
    )
    .await?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let updated = flat_maps
        .field_metadata_maps
        .find_by_universal_identifier(&merged.universal_identifier)
        .ok_or_else(|| GraphQlError::internal("Field metadata disappeared after update"))?;

    Ok(field_dto(updated, &flat_maps))
}

fn backs_exactly(
    index: &crate::metadata::flat::FlatIndexMetadata,
    field_universal_identifier: &str,
) -> bool {
    index.universal_flat_index_field_metadatas.len() == 1
        && index.universal_flat_index_field_metadatas[0]
            .sub_field_name
            .is_none()
        && index.universal_flat_index_field_metadatas[0].field_metadata_universal_identifier
            == field_universal_identifier
}

async fn delete_one_field(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);

    let id = require_update_id(&input)?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let existing = resolve_target_field(&id, &flat_maps.field_metadata_maps).map_err(|_| {
        GraphQlError::not_found("Field to delete not found")
            .with_extensions(json!({ "subCode": "FIELD_METADATA_NOT_FOUND" }))
    })?;

    let universal = flat_field_universal(&existing);

    if existing.application_universal_identifier
        == crate::metadata::names::TWENTY_STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER
        || existing.is_system
    {
        return Err(GraphQlError::forbidden(format!(
            "Cannot delete standard field \"{}\"",
            existing.name
        ))
        .with_extensions(json!({ "subCode": "FIELD_MUTATION_NOT_ALLOWED" })));
    }

    if existing.is_system_side_effect {
        return Err(GraphQlError::forbidden(format!(
            "Cannot delete system-managed field \"{}\"",
            existing.name
        ))
        .with_extensions(json!({ "subCode": "FIELD_MUTATION_NOT_ALLOWED" })));
    }

    let mut matrix = FlatEntityOperationsMatrix::default();

    add_to_delete(&mut matrix, field_entity(&universal));

    if let Some(target_identifier) = &existing.relation_target_field_metadata_universal_identifier {
        if let Some(inverse) = flat_maps
            .field_metadata_maps
            .find_by_universal_identifier(target_identifier)
        {
            add_to_delete(&mut matrix, field_entity(&flat_field_universal(inverse)));
        }
    }

    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    run_migration(
        ctx,
        matrix,
        &applications.custom_universal_identifier,
        "Multiple validation errors occurred while deleting field",
    )
    .await?;

    Ok(field_dto(&existing, &flat_maps))
}

async fn create_one_index(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);

    let index_input = input
        .get("index")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            GraphQlError::validation("Field \"index\" of type \"CreateIndexInput!\" is required.")
        })?;

    let object_metadata_id = super::input::require_string(index_input, "objectMetadataId")?;
    let object_metadata_id = require_uuid_like(&object_metadata_id)?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let Some(object) = flat_maps
        .object_metadata_maps
        .find_by_id(&object_metadata_id)
    else {
        return Err(GraphQlError::not_found(format!(
            "Object metadata {object_metadata_id} not found"
        ))
        .with_extensions(json!({ "subCode": "INDEX_OBJECT_NOT_FOUND" })));
    };

    let fields = index_input
        .get("fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if fields.is_empty() {
        return Err(
            GraphQlError::user_input("At least one field is required to create an index")
                .with_extensions(json!({ "subCode": "INDEX_FIELDS_REQUIRED" })),
        );
    }

    let index_type_raw = index_input
        .get("indexType")
        .and_then(Value::as_str)
        .unwrap_or("BTREE");

    let index_type = crate::metadata::flat::IndexType::parse(index_type_raw).ok_or_else(|| {
        GraphQlError::validation(format!(
            "Field \"indexType\" must be a valid IndexType, got {index_type_raw}"
        ))
    })?;

    let mut dedup_keys: Vec<String> = Vec::new();

    let mut index_field_entities: Vec<UniversalFlatIndexFieldMetadata> = Vec::new();

    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    for (order, entry) in fields.iter().enumerate() {
        let field_object = entry
            .as_object()
            .ok_or_else(|| GraphQlError::validation("Index fields must be objects"))?;

        let field_metadata_id = super::input::require_string(field_object, "fieldMetadataId")?;
        let field_metadata_id = require_uuid_like(&field_metadata_id)?;

        let sub_field_name = optional_string(field_object, "subFieldName")?;

        let dedup_key = format!(
            "{field_metadata_id}::{}",
            sub_field_name.clone().unwrap_or_default()
        );

        if dedup_keys.contains(&dedup_key) {
            return Err(
                GraphQlError::user_input("Duplicate field+sub-field in index field list")
                    .with_extensions(json!({ "subCode": "DUPLICATE_INDEX_FIELDS" })),
            );
        }

        dedup_keys.push(dedup_key);

        let Some(field) = flat_maps.field_metadata_maps.find_by_id(&field_metadata_id) else {
            return Err(GraphQlError::user_input(format!(
                "Field {field_metadata_id} not found on object {object_metadata_id}"
            ))
            .with_extensions(json!({ "subCode": "INDEX_FIELD_NOT_FOUND_ON_OBJECT" })));
        };

        if field.object_metadata_id.to_string() != object_metadata_id {
            return Err(GraphQlError::user_input(format!(
                "Field {} not found on object {object_metadata_id}",
                field.name
            ))
            .with_extensions(json!({ "subCode": "INDEX_FIELD_NOT_FOUND_ON_OBJECT" })));
        }

        let is_composite = matches!(
            field.field_type,
            FieldMetadataType::Address
                | FieldMetadataType::Currency
                | FieldMetadataType::Emails
                | FieldMetadataType::FullName
                | FieldMetadataType::Links
                | FieldMetadataType::Phones
        );

        if is_composite && sub_field_name.is_none() {
            return Err(GraphQlError::user_input(format!(
                "Composite field {} requires a sub-field selection",
                field.name
            ))
            .with_extensions(json!({ "subCode": "INDEX_NOT_SUPPORTED_FOR_COMPOSITE_FIELD" })));
        }

        if !is_composite && sub_field_name.is_some() {
            return Err(GraphQlError::user_input(format!(
                "Field {} is not composite — subFieldName must not be set",
                field.name
            ))
            .with_extensions(json!({ "subCode": "INDEX_NOT_SUPPORTED_FOR_COMPOSITE_FIELD" })));
        }

        if index_type == crate::metadata::flat::IndexType::Gin {
            if sub_field_name.is_some() {
                return Err(GraphQlError::user_input(format!(
                    "GIN index does not support composite sub-property {}.{}",
                    field.name,
                    sub_field_name.unwrap_or_default()
                ))
                .with_extensions(json!({ "subCode": "INDEX_TYPE_NOT_SUPPORTED_FOR_FIELD_TYPE" })));
            }

            if !super::service::GIN_COMPATIBLE_FIELD_TYPES.contains(&field.field_type.as_str()) {
                return Err(GraphQlError::user_input(format!(
                    "GIN index does not support field {} of type {}",
                    field.name,
                    field.field_type.as_str()
                ))
                .with_extensions(json!({ "subCode": "INDEX_TYPE_NOT_SUPPORTED_FOR_FIELD_TYPE" })));
            }
        }

        index_field_entities.push(UniversalFlatIndexFieldMetadata {
            order: order as i32,
            sub_field_name,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            index_metadata_universal_identifier: String::new(),
            field_metadata_universal_identifier: field.universal_identifier.clone(),
        });
    }

    let custom_index_count = flat_maps
        .index_metadata_maps
        .by_universal_identifier
        .values()
        .filter(|index| index.object_metadata_id == object.id && !index.is_system_side_effect)
        .count();

    if custom_index_count >= super::service::MAX_CUSTOM_INDEXES_PER_OBJECT {
        return Err(GraphQlError::conflict(format!(
            "Custom index limit of {} reached for object {object_metadata_id}",
            super::service::MAX_CUSTOM_INDEXES_PER_OBJECT
        ))
        .with_extensions(json!({ "subCode": "CUSTOM_INDEX_LIMIT_REACHED" })));
    }

    let columns: Vec<String> = index_field_entities
        .iter()
        .filter_map(|entity| {
            flat_maps
                .field_metadata_maps
                .find_by_universal_identifier(&entity.field_metadata_universal_identifier)
                .map(|field| {
                    if let Some(sub_field) = &entity.sub_field_name {
                        crate::metadata::names::compute_composite_column_name(
                            &field.name,
                            sub_field,
                        )
                    } else if field.field_type.is_relation_or_morph() {
                        crate::metadata::names::compute_relation_join_column_name(&field.name)
                    } else {
                        field.name.clone()
                    }
                })
        })
        .collect();

    let universal_identifier = Uuid::new_v4().to_string();

    for entity in &mut index_field_entities {
        entity.index_metadata_universal_identifier = universal_identifier.clone();
    }

    let now = chrono::Utc::now();

    let index = UniversalFlatIndexMetadata {
        universal_identifier: universal_identifier.clone(),
        application_universal_identifier: applications.custom_universal_identifier.clone(),
        name: super::service::index_name_for_columns(
            &flat_object_universal(object),
            &object.application_universal_identifier,
            &columns,
            false,
            None,
        ),
        object_metadata_universal_identifier: object.universal_identifier.clone(),
        is_unique: false,
        index_where_clause: None,
        index_type,
        is_system_side_effect: false,
        created_at: now,
        updated_at: now,
        universal_flat_index_field_metadatas: index_field_entities,
    };

    let mut matrix = FlatEntityOperationsMatrix::default();

    add_to_create(
        &mut matrix,
        UniversalFlatEntity::IndexMetadata(Box::new(index)),
    );

    run_migration(
        ctx,
        matrix,
        &applications.custom_universal_identifier,
        "Validation errors occurred while creating index",
    )
    .await?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let created = flat_maps
        .index_metadata_maps
        .find_by_universal_identifier(&universal_identifier)
        .ok_or_else(|| GraphQlError::internal("Created index metadata not found"))?;

    Ok(index_dto(created))
}

async fn delete_one_index(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let input = input_argument(ctx.selection, ctx.variables);

    let id = require_update_id(&input)?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let Some(index) = flat_maps.index_metadata_maps.find_by_id(&id) else {
        return Err(GraphQlError::not_found(format!("Index {id} not found"))
            .with_extensions(json!({ "subCode": "INDEX_NOT_FOUND" })));
    };

    if index.is_system_side_effect {
        return Err(GraphQlError::forbidden(format!(
            "Index {id} is a system index and cannot be deleted"
        ))
        .with_extensions(json!({ "subCode": "CANNOT_DELETE_SYSTEM_INDEX" })));
    }

    let mut matrix = FlatEntityOperationsMatrix::default();

    add_to_delete(
        &mut matrix,
        UniversalFlatEntity::IndexMetadata(Box::new(flat_index_universal(index))),
    );

    let applications = resolve_applications(ctx.state, ctx.workspace_id).await?;

    run_migration(
        ctx,
        matrix,
        &applications.custom_universal_identifier,
        "Validation errors occurred while deleting index",
    )
    .await?;

    Ok(index_dto(index))
}

pub const PUBLIC_WORKSPACE_DATA_FIELD: &str = "getPublicWorkspaceDataByDomain";

pub fn is_public_metadata_root_field(name: &str) -> bool {
    name == PUBLIC_WORKSPACE_DATA_FIELD
}

pub async fn dispatch_public(
    state: &AppState,
    selection: &FieldSelection,
    variables: &Value,
) -> ResponsePayload {
    if selection.name != PUBLIC_WORKSPACE_DATA_FIELD {
        return ResponsePayload::errors(vec![GraphQlError::validation(format!(
            "Cannot query field \"{}\" on type.",
            selection.name
        ))]);
    }
    match public_workspace_data(state, selection, variables).await {
        Ok(node) => ResponsePayload::data(json!({
            selection.response_key(): project(&node, &selection.selection)
        })),
        Err(error) => ResponsePayload::errors(vec![error]),
    }
}

fn origin_hostname(origin: &str) -> Option<String> {
    let authority = origin
        .trim()
        .split("://")
        .nth(1)
        .unwrap_or(origin.trim())
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let hostname = match authority.split_once(':') {
        Some((host, port)) if port.chars().all(|character| character.is_ascii_digit()) => host,
        _ => authority,
    };
    let hostname = hostname.trim_matches(|character| character == '[' || character == ']');
    if hostname.is_empty() {
        return None;
    }
    Some(hostname.to_lowercase())
}

pub fn workspace_labels_from_origin(origin: &str) -> (Option<String>, Option<String>) {
    let Some(hostname) = origin_hostname(origin) else {
        return (None, None);
    };
    if hostname == "localhost" || hostname.parse::<std::net::IpAddr>().is_ok() {
        return (None, None);
    }
    let Some((first_label, _)) = hostname.split_once('.') else {
        return (None, None);
    };
    if first_label.starts_with("ws-") {
        (Some(first_label.to_string()), None)
    } else {
        (None, Some(hostname))
    }
}

async fn public_workspace_data(
    state: &AppState,
    selection: &FieldSelection,
    variables: &Value,
) -> Result<Value, GraphQlError> {
    let origin = variables
        .get("origin")
        .and_then(Value::as_str)
        .or_else(|| selection.argument("origin").and_then(Value::as_str))
        .unwrap_or_default();
    let (subdomain, custom_domain) = workspace_labels_from_origin(origin);
    let row = if subdomain.is_some() || custom_domain.is_some() {
        sqlx::query_as::<_, (Uuid, String, Option<Uuid>)>(
            "SELECT id, \"displayName\", \"logoFileId\" FROM core.\"workspace\" \
             WHERE subdomain = $1 OR \"customDomain\" = $2 LIMIT 1",
        )
        .bind(&subdomain)
        .bind(&custom_domain)
        .fetch_optional(&state.core_pool)
        .await
        .map_err(|error| GraphQlError::internal(error.to_string()))?
    } else {
        sqlx::query_as::<_, (Uuid, String, Option<Uuid>)>(
            "SELECT id, \"displayName\", \"logoFileId\" FROM core.\"workspace\" \
             ORDER BY \"createdAt\" DESC LIMIT 1",
        )
        .fetch_optional(&state.core_pool)
        .await
        .map_err(|error| GraphQlError::internal(error.to_string()))?
    };
    let (workspace_id, display_name, logo_file_id) =
        row.ok_or_else(|| GraphQlError::not_found("Workspace not found"))?;
    let logo = match logo_file_id {
        Some(file_id) => {
            let signer = FileUrlSigner::new(
                state.jwt.clone(),
                state.config.server_url.clone(),
                state.config.file_token_expires_in_secs,
            );
            signer
                .signed_url(file_id, workspace_id)
                .await
                .map_err(|error| GraphQlError::from_auth_error(&error))?
        }
        None => String::new(),
    };
    Ok(json!({
        "id": workspace_id,
        "displayName": display_name,
        "logo": logo,
    }))
}

async fn current_user(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let auth = ctx
        .auth
        .ok_or_else(|| GraphQlError::unauthenticated("Authentication required"))?;
    let user_id = auth
        .user_id
        .ok_or_else(|| GraphQlError::unauthenticated("Authentication required"))?;
    let row = sqlx::query_as::<_, (Uuid, String, String, Option<String>, bool)>(
        "SELECT id, email, \"firstName\", \"lastName\", \"isEmailVerified\" \
         FROM core.\"user\" WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&ctx.state.core_pool)
    .await
    .map_err(|error| GraphQlError::internal(error.to_string()))?
    .ok_or_else(|| GraphQlError::not_found("User was not found"))?;
    let workspace = current_user_workspace_permissions_for_auth(ctx.state, auth)
        .await
        .map_err(GraphQlError::from)?;
    let mut record = json!({
        "id": row.0,
        "email": row.1,
        "firstName": row.2,
        "lastName": row.3,
        "isEmailVerified": row.4,
    });
    if let Some(workspace) = workspace {
        record["currentUserWorkspace"] = workspace;
    }
    Ok(record)
}

async fn list_objects(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    list_page(
        ctx,
        "objectMetadata",
        &[
            ("id", "id", "uuid"),
            ("universalIdentifier", "universalIdentifier", "uuid"),
            ("isActive", "isActive", "boolean"),
            ("isRemote", "isRemote", "boolean"),
            ("isSearchable", "isSearchable", "boolean"),
            ("isSystem", "isSystem", "boolean"),
            ("isUICreatable", "isUICreatable", "boolean"),
            ("isUIEditable", "isUIEditable", "boolean"),
            ("isUIReadOnly", "isUIEditable", "boolean"),
        ],
    )
    .await
}

async fn list_fields(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    list_page(
        ctx,
        "fieldMetadata",
        &[
            ("id", "id", "uuid"),
            ("isActive", "isActive", "boolean"),
            ("isSystem", "isSystem", "boolean"),
            ("isUIEditable", "isUIEditable", "boolean"),
            ("isUIReadOnly", "isUIEditable", "boolean"),
            ("objectMetadataId", "objectMetadataId", "uuid"),
        ],
    )
    .await
}

const DEFAULT_METADATA_PAGE_SIZE: i64 = 10;
const MAX_METADATA_PAGE_SIZE: i64 = 1000;

struct ParsedPaging {
    limit: i64,
    backward: bool,
    after_id: Option<Uuid>,
    before_id: Option<Uuid>,
}

fn parse_paging(paging: &Value) -> Result<ParsedPaging, GraphQlError> {
    let first = paging.get("first").and_then(Value::as_i64);
    let last = paging.get("last").and_then(Value::as_i64);
    let after = paging.get("after").and_then(Value::as_str);
    let before = paging.get("before").and_then(Value::as_str);

    if first.is_some() && last.is_some() {
        return Err(GraphQlError::user_input("Cannot use both first and last"));
    }

    if first.is_some() && before.is_some() {
        return Err(GraphQlError::user_input("Cannot use first with before"));
    }

    if last.is_some() && after.is_some() {
        return Err(GraphQlError::user_input("Cannot use last with after"));
    }

    if after.is_some() && before.is_some() {
        return Err(GraphQlError::user_input("Cannot use both after and before"));
    }

    if first.is_some_and(|value| value < 0) || last.is_some_and(|value| value < 0) {
        return Err(GraphQlError::user_input("Page size cannot be negative"));
    }

    let limit = first.or(last).unwrap_or(DEFAULT_METADATA_PAGE_SIZE);

    if limit > MAX_METADATA_PAGE_SIZE {
        return Err(GraphQlError::user_input(format!(
            "Requested page size of {limit} exceeds the maximum size of {MAX_METADATA_PAGE_SIZE}"
        )));
    }

    Ok(ParsedPaging {
        limit,
        backward: last.is_some() || before.is_some(),
        after_id: after.map(decode_cursor_id).transpose()?,
        before_id: before.map(decode_cursor_id).transpose()?,
    })
}

async fn list_page(
    ctx: &OperationContext<'_>,
    table: &str,
    filter_fields: &[(&str, &str, &str)],
) -> Result<Value, GraphQlError> {
    let filter = argument_value(ctx, "filter");
    let paging = argument_value(ctx, "paging");

    let pagination = parse_paging(&paging)?;

    let soft_delete = matches!(table, "application" | "view");

    let mut sql = format!(
        "SELECT id FROM core.\"{table}\" WHERE \"workspaceId\" = $1{}",
        if soft_delete {
            " AND \"deletedAt\" IS NULL"
        } else {
            ""
        }
    );

    let mut params: Vec<FilterParam> = Vec::new();

    if let Some(after) = pagination.after_id {
        sql.push_str(" AND id < $2");

        params.push(FilterParam::Uuid(after));
    }

    if let Some(before) = pagination.before_id {
        let placeholder = params.len() + 2;

        sql.push_str(&format!(" AND id > ${placeholder}"));

        params.push(FilterParam::Uuid(before));
    }

    push_filter_clauses(&filter, &mut sql, &mut params, filter_fields)?;

    let limit_index = params.len() + 2;

    let order = if pagination.backward { "ASC" } else { "DESC" };

    sql.push_str(&format!(" ORDER BY id {order} LIMIT ${limit_index}"));

    let mut rows_query = sqlx::query_as::<_, (Uuid,)>(&sql).bind(ctx.workspace_id);

    for param in &params {
        match param {
            FilterParam::Uuid(value) => rows_query = rows_query.bind(value),
            FilterParam::Scalar(Value::Bool(raw)) => rows_query = rows_query.bind(*raw),
            FilterParam::Scalar(value) if value.as_i64().is_some() => {
                rows_query = rows_query.bind(value.as_i64().unwrap_or_default())
            }
            FilterParam::Scalar(value) if value.as_f64().is_some() => {
                rows_query = rows_query.bind(value.as_f64().unwrap_or_default())
            }
            FilterParam::Scalar(value) if value.is_string() => {
                rows_query = rows_query.bind(value.as_str().unwrap_or_default().to_string())
            }
            FilterParam::Scalar(value) => rows_query = rows_query.bind(value.to_string()),
        }
    }

    rows_query = rows_query.bind(pagination.limit + 1);

    let rows = rows_query
        .fetch_all(&ctx.state.core_pool)
        .await
        .map_err(|error| GraphQlError::internal(error.to_string()))?;

    let has_more_items = rows.len() as i64 > pagination.limit;

    let mut page: Vec<Uuid> = rows
        .into_iter()
        .take(pagination.limit as usize)
        .map(|(id,)| id)
        .collect();

    if pagination.backward {
        page.reverse();
    }

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    let nodes: Vec<Value> = match table {
        "objectMetadata" => page
            .iter()
            .filter_map(|id| flat_maps.object_metadata_maps.find_by_id(&id.to_string()))
            .map(object_dto)
            .collect(),
        _ => page
            .iter()
            .filter_map(|id| flat_maps.field_metadata_maps.find_by_id(&id.to_string()))
            .map(|field| field_dto(field, &flat_maps))
            .collect(),
    };

    Ok(cursor_connection(&nodes, &pagination, has_more_items))
}

async fn get_object(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let id = argument_value(ctx, "id");

    let id = id
        .as_str()
        .ok_or_else(|| GraphQlError::validation("id must be a string"))?;

    let id = require_uuid_like(id)?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    flat_maps
        .object_metadata_maps
        .find_by_id(&id)
        .map(object_dto)
        .ok_or_else(|| {
            GraphQlError::not_found(format!("Unable to find ObjectMetadataEntity with id: {id}"))
        })
}

async fn get_field(ctx: &OperationContext<'_>) -> Result<Value, GraphQlError> {
    let id = argument_value(ctx, "id");

    let id = id
        .as_str()
        .ok_or_else(|| GraphQlError::validation("id must be a string"))?;

    let id = require_uuid_like(id)?;

    let flat_maps = load_flat_maps(ctx.state, ctx.workspace_id).await?;

    flat_maps
        .field_metadata_maps
        .find_by_id(&id)
        .map(|field| field_dto(field, &flat_maps))
        .ok_or_else(|| {
            GraphQlError::not_found(format!("Unable to find FieldMetadataEntity with id: {id}"))
        })
}

fn argument_value(ctx: &OperationContext<'_>, name: &str) -> Value {
    ctx.selection
        .argument(name)
        .cloned()
        .or_else(|| ctx.variables.get(name).cloned())
        .unwrap_or_else(|| json!({}))
}

fn decode_cursor_id(cursor: &str) -> Result<Uuid, GraphQlError> {
    let decoded = base64url_decode(cursor)
        .ok_or_else(|| GraphQlError::user_input(format!("Invalid cursor: {cursor}")))?;

    let value: Value = serde_json::from_slice(&decoded)
        .map_err(|_| GraphQlError::user_input(format!("Invalid cursor: {cursor}")))?;

    let id = value
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| GraphQlError::user_input(format!("Invalid cursor: {cursor}")))?;

    Uuid::parse_str(id).map_err(|_| GraphQlError::user_input(format!("Invalid cursor: {cursor}")))
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;

    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(input)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(input))
        .ok()
}

fn base64url_encode(bytes: &[u8]) -> String {
    use base64::Engine;

    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn cursor_connection(nodes: &[Value], pagination: &ParsedPaging, has_more_items: bool) -> Value {
    let edges: Vec<Value> = nodes
        .iter()
        .map(|node| {
            let id = node.get("id").and_then(Value::as_str).unwrap_or_default();

            json!({
                "node": node,
                "cursor": base64url_encode(json!({ "id": id }).to_string().as_bytes())
            })
        })
        .collect();

    let has_next_page = if pagination.backward {
        pagination.before_id.is_some()
    } else {
        has_more_items
    };

    let has_previous_page = if pagination.backward {
        has_more_items
    } else {
        pagination.after_id.is_some()
    };

    json!({
        "edges": edges,
        "pageInfo": {
            "hasNextPage": has_next_page,
            "hasPreviousPage": has_previous_page,
            "startCursor": edges.first().and_then(|edge| edge.get("cursor")).cloned().unwrap_or(Value::Null),
            "endCursor": edges.last().and_then(|edge| edge.get("cursor")).cloned().unwrap_or(Value::Null)
        },
        "totalCount": nodes.len()
    })
}

enum FilterParam {
    Uuid(Uuid),
    Scalar(Value),
}

enum FilterCondition {
    Comparison {
        column: String,
        kind: String,
        operator: String,
        value: Value,
        inverted_boolean: bool,
    },
    Nullity {
        column: String,
        negated: bool,
    },
    In {
        column: String,
        kind: String,
        values: Vec<Value>,
        negated: bool,
    },
    AlwaysFalse,
    And(Vec<FilterCondition>),
    Or(Vec<FilterCondition>),
}

fn build_filter_conditions(
    filter: &Value,
    columns: &[(&str, &str, &str)],
) -> Result<Vec<FilterCondition>, GraphQlError> {
    let Some(object) = filter.as_object() else {
        return Ok(Vec::new());
    };

    let mut conditions = Vec::new();

    for (field, value) in object {
        if field == "and" {
            let Some(items) = value.as_array() else {
                return Err(GraphQlError::user_input(
                    "Filter operator \"and\" expects an array of filters",
                ));
            };

            let mut branches = Vec::new();

            for item in items {
                branches.extend(build_filter_conditions(item, columns)?);
            }

            if !branches.is_empty() {
                conditions.push(FilterCondition::And(branches));
            }

            continue;
        }

        if field == "or" {
            let Some(items) = value.as_array() else {
                return Err(GraphQlError::user_input(
                    "Filter operator \"or\" expects an array of filters",
                ));
            };

            let mut branches = Vec::new();

            for item in items {
                let inner = build_filter_conditions(item, columns)?;

                if inner.is_empty() {
                    continue;
                }

                if inner.len() == 1 {
                    branches.push(
                        inner
                            .into_iter()
                            .next()
                            .unwrap_or(FilterCondition::AlwaysFalse),
                    );
                } else {
                    branches.push(FilterCondition::And(inner));
                }
            }

            if !branches.is_empty() {
                conditions.push(FilterCondition::Or(branches));
            }

            continue;
        }

        let Some((_, column, kind)) = columns.iter().find(|(name, _, _)| *name == field) else {
            return Err(GraphQlError::user_input(format!(
                "Unknown filter field: {field}"
            )));
        };

        let Some(comparison) = value.as_object() else {
            return Err(GraphQlError::user_input(format!(
                "Filter value for {field} must be an object"
            )));
        };

        for (operator, comparison_value) in comparison {
            let condition = match operator.as_str() {
                "eq" | "neq" | "gt" | "gte" | "lt" | "lte" | "like" | "notLike" | "iLike"
                | "notILike" => {
                    let sql_operator = match operator.as_str() {
                        "eq" => "=",
                        "neq" => "!=",
                        "gt" => ">",
                        "gte" => ">=",
                        "lt" => "<",
                        "lte" => "<=",
                        "like" => "LIKE",
                        "notLike" => "NOT LIKE",
                        "iLike" => "ILIKE",
                        _ => "NOT ILIKE",
                    };

                    if *kind == "boolean" && !comparison_value.is_boolean() {
                        return Err(GraphQlError::user_input(format!(
                            "Filter value for {field} must be a boolean"
                        )));
                    }

                    if *kind == "uuid" && !comparison_value.is_string() {
                        return Err(GraphQlError::user_input(format!(
                            "Filter value for {field} must be a uuid"
                        )));
                    }

                    FilterCondition::Comparison {
                        column: (*column).to_string(),
                        kind: (*kind).to_string(),
                        operator: sql_operator.to_string(),
                        value: comparison_value.clone(),
                        inverted_boolean: field == "isUIReadOnly",
                    }
                }
                "is" | "isNot" => {
                    if comparison_value.is_null() {
                        FilterCondition::Nullity {
                            column: (*column).to_string(),
                            negated: operator == "isNot",
                        }
                    } else {
                        FilterCondition::Comparison {
                            column: (*column).to_string(),
                            kind: (*kind).to_string(),
                            operator: if operator == "is" { "=" } else { "!=" }.to_string(),
                            value: comparison_value.clone(),
                            inverted_boolean: field == "isUIReadOnly",
                        }
                    }
                }
                "in" | "notIn" => {
                    let Some(items) = comparison_value.as_array() else {
                        return Err(GraphQlError::user_input(format!(
                            "Filter operator \"{operator}\" expects an array for {field}"
                        )));
                    };

                    if items.is_empty() && operator == "in" {
                        FilterCondition::AlwaysFalse
                    } else {
                        FilterCondition::In {
                            column: (*column).to_string(),
                            kind: (*kind).to_string(),
                            values: items.clone(),
                            negated: operator == "notIn",
                        }
                    }
                }
                other => {
                    return Err(GraphQlError::user_input(format!(
                        "Unknown filter operator: {other}"
                    )))
                }
            };

            conditions.push(condition);
        }
    }

    Ok(conditions)
}

fn push_filter_clauses(
    filter: &Value,
    sql: &mut String,
    params: &mut Vec<FilterParam>,
    columns: &[(&str, &str, &str)],
) -> Result<(), GraphQlError> {
    let conditions = build_filter_conditions(filter, columns)?;

    for condition in &conditions {
        sql.push_str(" AND ");

        emit_condition(condition, sql, params)?;
    }

    Ok(())
}

fn emit_condition(
    condition: &FilterCondition,
    sql: &mut String,
    params: &mut Vec<FilterParam>,
) -> Result<(), GraphQlError> {
    match condition {
        FilterCondition::AlwaysFalse => {
            sql.push_str("1 = 0");

            Ok(())
        }
        FilterCondition::Nullity { column, negated } => {
            let predicate = if *negated { "IS NOT NULL" } else { "IS NULL" };

            sql.push_str(&format!("\"{column}\" {predicate}"));

            Ok(())
        }
        FilterCondition::Comparison {
            column,
            kind,
            operator,
            value,
            inverted_boolean,
        } => {
            let rendered = if *inverted_boolean {
                value
                    .as_bool()
                    .map(|raw| Value::Bool(!raw))
                    .unwrap_or_else(|| value.clone())
            } else {
                value.clone()
            };

            let parameter_index = params.len() + 2;

            push_param(params, kind, &rendered, column)?;

            sql.push_str(&format!("\"{column}\" {operator} ${parameter_index}"));

            Ok(())
        }
        FilterCondition::In {
            column,
            kind,
            values,
            negated,
        } => {
            if values.is_empty() {
                sql.push_str(if *negated { "1 = 1" } else { "1 = 0" });

                return Ok(());
            }

            let predicate = if *negated { "NOT IN" } else { "IN" };

            sql.push_str(&format!("\"{column}\" {predicate} ("));

            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    sql.push_str(", ");
                }

                let parameter_index = params.len() + 2;

                push_param(params, kind, item, column)?;

                sql.push_str(&format!("${parameter_index}"));
            }

            sql.push(')');

            Ok(())
        }
        FilterCondition::And(branches) => emit_group("AND", branches, sql, params),
        FilterCondition::Or(branches) => emit_group("OR", branches, sql, params),
    }
}

fn emit_group(
    joiner: &str,
    branches: &[FilterCondition],
    sql: &mut String,
    params: &mut Vec<FilterParam>,
) -> Result<(), GraphQlError> {
    if branches.is_empty() {
        sql.push_str("1 = 1");

        return Ok(());
    }

    sql.push('(');

    for (index, branch) in branches.iter().enumerate() {
        if index > 0 {
            sql.push_str(&format!(" {joiner} "));
        }

        emit_condition(branch, sql, params)?;
    }

    sql.push(')');

    Ok(())
}

fn push_param(
    params: &mut Vec<FilterParam>,
    kind: &str,
    value: &Value,
    column: &str,
) -> Result<(), GraphQlError> {
    if kind == "uuid" {
        let raw = value.as_str().ok_or_else(|| {
            GraphQlError::user_input(format!("Filter value for {column} must be a uuid"))
        })?;

        let parsed = Uuid::parse_str(raw).map_err(|_| {
            GraphQlError::user_input(format!("Filter value for {column} must be a uuid"))
        })?;

        params.push(FilterParam::Uuid(parsed));

        return Ok(());
    }

    if value.is_null() {
        return Err(GraphQlError::user_input(format!(
            "Filter value for {column} must not be null"
        )));
    }

    params.push(FilterParam::Scalar(value.clone()));

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::add_to_create;
    use super::field_entity;
    use super::is_public_metadata_root_field;
    use super::object_entity;
    use super::origin_hostname;
    use super::workspace_labels_from_origin;
    use crate::metadata::actions::MetadataName;
    use crate::metadata::actions::UniversalAction;
    use crate::metadata::actions::UniversalFlatEntity;
    use crate::metadata::api::service::build_options;
    use crate::metadata::api::service::compute_api_from_to_maps;
    use crate::metadata::api::service::infer_deletion_for_deletes;

    #[test]
    fn the_public_workspace_field_is_the_only_unauthenticated_root() {
        assert!(is_public_metadata_root_field(
            "getPublicWorkspaceDataByDomain"
        ));
        assert!(!is_public_metadata_root_field("currentUser"));
        assert!(!is_public_metadata_root_field("objects"));
    }

    #[test]
    fn origin_hostnames_drop_scheme_port_and_path() {
        assert_eq!(
            origin_hostname("http://127.0.0.1:3001/objects/people").as_deref(),
            Some("127.0.0.1")
        );
        assert_eq!(
            origin_hostname("https://App.Example.COM:8443/x").as_deref(),
            Some("app.example.com")
        );
        assert_eq!(
            origin_hostname("not-an-origin").as_deref(),
            Some("not-an-origin")
        );
        assert_eq!(origin_hostname("").as_deref(), None);
        assert_eq!(origin_hostname("http://").as_deref(), None);
    }

    #[test]
    fn workspace_subdomain_origins_resolve_the_workspace_subdomain() {
        assert_eq!(
            workspace_labels_from_origin("http://ws-abc.localhost:3001"),
            (Some("ws-abc".to_string()), None)
        );
    }

    #[test]
    fn bare_hosts_fall_back_to_the_default_workspace_lookup() {
        for origin in [
            "http://127.0.0.1:3001",
            "http://localhost:3001",
            "http://localhost",
        ] {
            assert_eq!(workspace_labels_from_origin(origin), (None, None));
        }
    }

    #[test]
    fn non_workspace_hosts_are_treated_as_custom_domains() {
        assert_eq!(
            workspace_labels_from_origin("https://crm.acme.com/welcome"),
            (None, Some("crm.acme.com".to_string()))
        );
        assert_eq!(
            workspace_labels_from_origin("https://acme.com"),
            (None, Some("acme.com".to_string()))
        );
    }
    use crate::metadata::api::service::normalize_create_action_entities;
    use crate::metadata::api::service::transpile_create_object;
    use crate::metadata::api::service::ResolvedApplications;
    use crate::metadata::builders::build_workspace_migration;
    use crate::metadata::builders::expand_matrix_with_side_effects;
    use crate::metadata::builders::FlatEntityOperationsMatrix;
    use crate::metadata::builders::WorkspaceMigrationBuildResult;
    use crate::metadata::flat::UniversalWorkspaceMaps;

    fn applications() -> ResolvedApplications {
        let custom_universal_identifier = uuid::Uuid::new_v4().to_string();

        ResolvedApplications {
            standard_universal_identifier: custom_universal_identifier.clone(),
            custom_universal_identifier,
            custom_id: uuid::Uuid::new_v4(),
        }
    }

    fn create_one_object_migration(applications: &ResolvedApplications) -> CreateCase {
        let input = json!({
            "object": {
                "nameSingular": "rocket",
                "namePlural": "rockets",
                "labelSingular": "Rocket",
                "labelPlural": "Rockets"
            }
        });

        let transpilation = transpile_create_object(&input, applications)
            .expect("transpiling the rocket object creation");

        let name_field_universal_identifier = transpilation
            .name_field
            .as_ref()
            .expect("the name field is part of the creation")
            .universal_identifier
            .clone();

        let object_universal_identifier = transpilation.entity.universal_identifier.clone();

        let mut matrix = FlatEntityOperationsMatrix::default();

        add_to_create(&mut matrix, object_entity(&transpilation.entity));

        add_to_create(&mut matrix, field_entity(name_field(&transpilation)));

        let universal_maps = UniversalWorkspaceMaps::default();

        let expansion = expand_matrix_with_side_effects(matrix, &universal_maps);

        let infer_deletion = infer_deletion_for_deletes(&expansion.matrix);
        let from_to = compute_api_from_to_maps(&universal_maps, &expansion.matrix);
        let options = build_options(
            applications,
            &applications.custom_universal_identifier,
            infer_deletion,
        );

        let migration = match build_workspace_migration(options, from_to) {
            WorkspaceMigrationBuildResult::Success(migration) => migration,
            WorkspaceMigrationBuildResult::Fail(report) => {
                panic!("building the rocket migration failed: {report:?}")
            }
        };

        CreateCase {
            migration,
            object_universal_identifier,
            name_field_universal_identifier,
        }
    }

    fn name_field(
        transpilation: &crate::metadata::api::service::ObjectCreateTranspilation,
    ) -> &crate::metadata::flat::UniversalFlatFieldMetadata {
        transpilation
            .name_field
            .as_ref()
            .expect("the name field is part of the creation")
    }

    struct CreateCase {
        migration: crate::metadata::actions::WorkspaceMigration,
        object_universal_identifier: String,
        name_field_universal_identifier: String,
    }

    fn object_create(
        migration: &crate::metadata::actions::WorkspaceMigration,
    ) -> &crate::metadata::actions::UniversalCreateAction {
        let mut objects: Vec<&crate::metadata::actions::UniversalCreateAction> = migration
            .actions
            .iter()
            .filter_map(|action| match action {
                UniversalAction::Create(create)
                    if create.metadata_name == MetadataName::ObjectMetadata =>
                {
                    Some(create)
                }
                _ => None,
            })
            .collect();

        assert_eq!(objects.len(), 1, "exactly one object create action");

        objects.remove(0)
    }

    #[test]
    fn the_object_create_carries_a_typed_flat_entity_for_the_runner() {
        let applications = applications();

        let case = create_one_object_migration(&applications);

        let mut migration = case.migration;

        normalize_create_action_entities(&mut migration.actions);

        let create = object_create(&migration);

        let object = match create.flat_entity.as_ref() {
            Some(UniversalFlatEntity::ObjectMetadata(object)) => object.as_ref(),
            other => panic!("object create lost its typed flat entity: {other:?}"),
        };

        assert_eq!(object.name_singular, "rocket");
        assert_eq!(object.name_plural, "rockets");
        assert_eq!(object.label_singular, "Rocket");
        assert_eq!(object.label_plural, "Rockets");
        assert_eq!(
            object.universal_identifier,
            case.object_universal_identifier
        );

        assert_eq!(
            object
                .label_identifier_field_metadata_universal_identifier
                .as_deref(),
            Some(case.name_field_universal_identifier.as_str())
        );

        let name_field = create
            .universal_flat_field_metadatas
            .iter()
            .find(|field| field.name == "name")
            .expect("the name field rides the object create");

        assert_eq!(
            name_field.universal_identifier,
            case.name_field_universal_identifier
        );

        assert!(create
            .field_id_by_universal_identifier
            .as_ref()
            .map(|ids| ids.contains_key(&name_field.universal_identifier))
            .unwrap_or(false));

        assert!(create.id.is_some());

        let standalone_name = migration.actions.iter().any(|action| match action {
            UniversalAction::Create(create) => {
                create.metadata_name == MetadataName::FieldMetadata
                    && matches!(
                        create.flat_entity.as_ref(),
                        Some(UniversalFlatEntity::FieldMetadata(field)) if field.name == "name"
                    )
            }
            _ => false,
        });

        assert!(
            !standalone_name,
            "the name field is not also emitted as a standalone field create"
        );
    }

    #[test]
    fn the_builder_downgrades_create_entities_and_normalization_restores_them() {
        let applications = applications();

        let case = create_one_object_migration(&applications);

        let mut migration = case.migration;

        let raw = object_create(&migration);

        assert!(
            matches!(raw.flat_entity, Some(UniversalFlatEntity::Other { .. })),
            "strip_create_foreign_key_aggregators leaves the create entity as Other"
        );

        normalize_create_action_entities(&mut migration.actions);

        let create = object_create(&migration);

        assert!(
            matches!(
                create.flat_entity,
                Some(UniversalFlatEntity::ObjectMetadata(_))
            ),
            "normalization re-types the create entity for the runner"
        );
    }

    #[test]
    fn the_object_create_drops_foreign_key_aggregators() {
        let applications = applications();

        let case = create_one_object_migration(&applications);

        let mut migration = case.migration;

        normalize_create_action_entities(&mut migration.actions);

        let create = object_create(&migration);

        let Some(UniversalFlatEntity::ObjectMetadata(object)) = create.flat_entity.as_ref() else {
            panic!("expected a typed object entity");
        };

        assert!(object.field_universal_identifiers.is_empty());
        assert!(object.view_universal_identifiers.is_empty());
    }

    #[test]
    fn the_create_expansion_seeds_no_view_or_navigation_entities() {
        let applications = applications();

        let input = json!({
            "object": {
                "nameSingular": "probe",
                "namePlural": "probes",
                "labelSingular": "Probe",
                "labelPlural": "Probes"
            }
        });

        let transpilation = transpile_create_object(&input, &applications).unwrap();

        let mut matrix = FlatEntityOperationsMatrix::default();

        add_to_create(&mut matrix, object_entity(&transpilation.entity));
        add_to_create(&mut matrix, field_entity(name_field(&transpilation)));

        let expansion = expand_matrix_with_side_effects(matrix, &UniversalWorkspaceMaps::default());

        let mut names: Vec<&str> = expansion
            .matrix
            .by_metadata_name
            .iter()
            .filter(|(_, bucket)| !bucket.to_create.is_empty() || !bucket.to_update.is_empty())
            .map(|(metadata_name, _)| metadata_name.as_str())
            .collect();

        names.sort_unstable();

        assert_eq!(
            names,
            vec!["fieldMetadata", "objectMetadata"],
            "wave 3 ports the object + name field only; views, favorites and \
             navigation are side-effect services outside this scope"
        );

        let case = create_one_object_migration(&applications);

        let mut migration = case.migration;

        normalize_create_action_entities(&mut migration.actions);

        let unsupported: Vec<MetadataName> = migration
            .actions
            .iter()
            .filter_map(|action| match action {
                UniversalAction::Create(create)
                    if !matches!(
                        create.metadata_name,
                        MetadataName::ObjectMetadata
                            | MetadataName::FieldMetadata
                            | MetadataName::Index
                            | MetadataName::FieldMetadataOption
                            | MetadataName::SearchFieldMetadata
                    ) =>
                {
                    Some(create.metadata_name)
                }
                _ => None,
            })
            .collect();

        assert!(unsupported.is_empty(), "no action names the runner rejects");
    }
}
