#![allow(dead_code)]

pub mod definitions;
pub mod permissions;

use serde_json::Value;
use uuid::Uuid;

use crate::metadata::actions::MetadataName;
use crate::metadata::actions::UniversalAction;
use crate::metadata::actions::UniversalCreateAction;
use crate::metadata::actions::UniversalFlatEntity;
use crate::metadata::actions::WorkspaceMigration;
use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::IndexType;
use crate::metadata::flat::UniversalFlatFieldMetadata;
use crate::metadata::flat::UniversalFlatFieldMetadataOption;
use crate::metadata::flat::UniversalFlatIndexFieldMetadata;
use crate::metadata::flat::UniversalFlatIndexMetadata;
use crate::metadata::flat::UniversalFlatObjectMetadata;
use crate::metadata::flat::UniversalFlatSearchFieldMetadata;
use crate::state::AppState;
use crate::workspace_migrations::runner::run_workspace_migration;
use crate::workspace_migrations::runner::RunnerError;
use crate::workspace_migrations::runner::RunnerInput;
use crate::workspace_migrations::runner::RunnerOutcome;

pub use definitions::STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER;

const STANDARD_APPLICATION_NAME: &str = "Standard";
const STANDARD_APPLICATION_SOURCE_PATH: &str = "cli-sync";
const STANDARD_APPLICATION_VERSION: &str = "1.0.1";
const SEARCH_VECTOR_FIELD_NAME: &str = "searchVector";

#[derive(Debug)]
pub struct SeedError {
    pub code: &'static str,
    pub message: String,
}

impl SeedError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_definition(message: impl Into<String>) -> Self {
        Self::new("INVALID_STANDARD_OBJECT_DEFINITION", message)
    }

    pub fn runner(error: RunnerError) -> Self {
        Self::new("STANDARD_OBJECT_SEED_FAILED", error.to_string())
    }
}

impl From<sqlx::Error> for SeedError {
    fn from(error: sqlx::Error) -> Self {
        Self::new("STANDARD_OBJECT_SEED_FAILED", error.to_string())
    }
}

impl From<RunnerError> for SeedError {
    fn from(error: RunnerError) -> Self {
        Self::runner(error)
    }
}

#[derive(Debug, Default)]
pub struct SeedOutcome {
    pub seeded: bool,
    pub objects: usize,
    pub fields: usize,
    pub indexes: usize,
    pub runner: Option<RunnerOutcome>,
}

pub async fn is_seeded(state: &AppState, workspace_id: Uuid) -> Result<bool, SeedError> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM core.\"objectMetadata\" \
         WHERE \"workspaceId\" = $1 AND \"applicationId\" IN ( \
             SELECT id FROM core.\"application\" WHERE \"workspaceId\" = $1 \
             AND \"universalIdentifier\" = $2 \
         ))",
    )
    .bind(workspace_id)
    .bind(
        Uuid::parse_str(STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER)
            .expect("standard application universal identifier is a uuid"),
    )
    .fetch_one(&state.core_pool)
    .await?;

    Ok(row.0)
}

async fn ensure_standard_application(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<Uuid, SeedError> {
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM core.\"application\" \
         WHERE \"workspaceId\" = $1 AND \"universalIdentifier\" = $2",
    )
    .bind(workspace_id)
    .bind(
        Uuid::parse_str(STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER)
            .expect("standard application universal identifier is a uuid"),
    )
    .fetch_optional(&state.core_pool)
    .await?;

    if let Some((id,)) = existing {
        return Ok(id);
    }

    let application_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO core.\"application\" \
         (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", version, \
          \"canBeUninstalled\", \"workspaceId\") \
         VALUES ($1, $2, $3, 'local', 'INSTALLED', $4, $5, false, $6)",
    )
    .bind(application_id)
    .bind(
        Uuid::parse_str(STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER)
            .expect("standard application universal identifier is a uuid"),
    )
    .bind(STANDARD_APPLICATION_NAME)
    .bind(STANDARD_APPLICATION_SOURCE_PATH)
    .bind(STANDARD_APPLICATION_VERSION)
    .bind(workspace_id)
    .execute(&state.core_pool)
    .await?;

    Ok(application_id)
}

fn parse_json(raw: Option<&str>) -> Result<Option<Value>, SeedError> {
    match raw {
        Some(text) => serde_json::from_str(text)
            .map(Some)
            .map_err(|error| SeedError::invalid_definition(error.to_string())),
        None => Ok(None),
    }
}

fn parse_field_type(raw: &str) -> Result<FieldMetadataType, SeedError> {
    FieldMetadataType::parse(raw)
        .ok_or_else(|| SeedError::invalid_definition(format!("unknown field type {raw}")))
}

fn parse_index_type(raw: &str) -> Result<IndexType, SeedError> {
    IndexType::parse(raw)
        .ok_or_else(|| SeedError::invalid_definition(format!("unknown index type {raw}")))
}

fn field_from_definition(
    field: &definitions::StandardFieldDefinition,
) -> Result<UniversalFlatFieldMetadata, SeedError> {
    Ok(UniversalFlatFieldMetadata {
        universal_identifier: field.universal_identifier.to_string(),
        application_universal_identifier: STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER.to_string(),
        field_type: parse_field_type(field.field_type)?,
        name: field.name.to_string(),
        label: field.label.to_string(),
        description: field.description.map(String::from),
        icon: field.icon.map(String::from),
        is_active: field.is_active,
        is_system: field.is_system,
        is_system_side_effect: field.is_system_side_effect,
        is_nullable: field.is_nullable,
        is_unique: false,
        is_searchable: false,
        is_audit_logged: field.is_audit_logged,
        is_ui_editable: field.is_ui_editable,
        disabled_field_permission_actions: None,
        is_label_synced_with_name: field.is_label_synced_with_name,
        overrides: parse_json(field.overrides_json)?,
        default_value: parse_json(field.default_value_json)?,
        universal_settings: parse_json(field.settings_json)?,
        options: parse_json(field.options_json)?,
        morph_id: match field.morph_id {
            Some(raw) => Some(
                Uuid::parse_str(raw)
                    .map_err(|error| SeedError::invalid_definition(error.to_string()))?,
            ),
            None => None,
        },
        relation_target_field_metadata_universal_identifier: field
            .relation_target_field_universal_identifier
            .map(String::from),
        relation_target_object_metadata_universal_identifier: field
            .relation_target_object_universal_identifier
            .map(String::from),
        ..UniversalFlatFieldMetadata::default()
    })
}

fn object_create_action(
    object: &definitions::StandardObjectDefinition,
    fields: Vec<UniversalFlatFieldMetadata>,
    object_id: Uuid,
    field_id_map: std::collections::HashMap<String, String>,
) -> UniversalAction {
    UniversalAction::Create(UniversalCreateAction {
        metadata_name: MetadataName::ObjectMetadata,
        id: Some(object_id.to_string()),
        flat_entity: Some(UniversalFlatEntity::ObjectMetadata(Box::new(
            UniversalFlatObjectMetadata {
                universal_identifier: object.universal_identifier.to_string(),
                application_universal_identifier: STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER
                    .to_string(),
                name_singular: object.name_singular.to_string(),
                name_plural: object.name_plural.to_string(),
                label_singular: object.label_singular.to_string(),
                label_plural: object.label_plural.to_string(),
                description: object.description.map(String::from),
                icon: object.icon.map(String::from),
                color: object.color.map(String::from),
                is_remote: false,
                is_active: object.is_active,
                is_system: object.is_system,
                is_searchable: object.is_searchable,
                is_audit_logged: object.is_audit_logged,
                is_ui_editable: true,
                is_ui_creatable: true,
                disabled_object_permission_actions: None,
                open_record_in: object.open_record_in.and_then(parse_open_record_in),
                is_label_synced_with_name: object.is_label_synced_with_name,
                overrides: parse_json(object.overrides_json).ok().flatten(),
                duplicate_criteria: parse_json(object.duplicate_criteria_json).ok().flatten(),
                shortcut: object.shortcut.map(String::from),
                label_identifier_field_metadata_universal_identifier: object
                    .label_identifier_field_universal_identifier
                    .map(String::from),
                image_identifier_field_metadata_universal_identifier: object
                    .image_identifier_field_universal_identifier
                    .map(String::from),
                target_table_name: Some(object.target_table_name.to_string()),
                ..UniversalFlatObjectMetadata::default()
            },
        ))),
        universal_flat_field_metadatas: fields,
        field_id_by_universal_identifier: Some(field_id_map),
        column_plan: Some(
            object
                .column_plan
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
        ),
        ..UniversalCreateAction::default()
    })
}

fn parse_open_record_in(raw: &str) -> Option<crate::metadata::flat::ObjectOpenRecordIn> {
    match raw {
        "SIDE_PANEL" => Some(crate::metadata::flat::ObjectOpenRecordIn::SidePanel),
        "RECORD_PAGE" => Some(crate::metadata::flat::ObjectOpenRecordIn::RecordPage),
        "USER_CHOICE" => Some(crate::metadata::flat::ObjectOpenRecordIn::UserChoice),
        _ => None,
    }
}

fn field_create_action(
    field: UniversalFlatFieldMetadata,
    object_universal_identifier: &str,
    field_id: Uuid,
) -> UniversalAction {
    let mut field = field;
    field.object_metadata_universal_identifier = object_universal_identifier.to_string();

    UniversalAction::Create(UniversalCreateAction {
        metadata_name: MetadataName::FieldMetadata,
        id: Some(field_id.to_string()),
        flat_entity: Some(UniversalFlatEntity::FieldMetadata(Box::new(field))),
        ..UniversalCreateAction::default()
    })
}

fn is_relation_field(field: &definitions::StandardFieldDefinition) -> bool {
    matches!(field.field_type, "RELATION" | "MORPH_RELATION")
}

fn option_create_action(option: &definitions::StandardFieldOptionDefinition) -> UniversalAction {
    UniversalAction::Create(UniversalCreateAction {
        metadata_name: MetadataName::FieldMetadataOption,
        flat_entity: Some(UniversalFlatEntity::FieldMetadataOption(Box::new(
            UniversalFlatFieldMetadataOption {
                universal_identifier: option.universal_identifier.to_string(),
                application_universal_identifier: STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER
                    .to_string(),
                field_metadata_universal_identifier: option.field_universal_identifier.to_string(),
                value: option.value.to_string(),
                label: option.label.to_string(),
                color: option.color.map(String::from),
                position: option.position,
                is_active: true,
                ..UniversalFlatFieldMetadataOption::default()
            },
        ))),
        ..UniversalCreateAction::default()
    })
}

fn search_field_create_action(
    search_field: &definitions::StandardSearchFieldDefinition,
    id: Uuid,
) -> UniversalAction {
    UniversalAction::Create(UniversalCreateAction {
        metadata_name: MetadataName::SearchFieldMetadata,
        id: Some(id.to_string()),
        flat_entity: Some(UniversalFlatEntity::SearchFieldMetadata(Box::new(
            UniversalFlatSearchFieldMetadata {
                universal_identifier: search_field.universal_identifier.to_string(),
                application_universal_identifier: STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER
                    .to_string(),
                object_metadata_universal_identifier: search_field
                    .object_universal_identifier
                    .to_string(),
                field_metadata_universal_identifier: search_field
                    .field_universal_identifier
                    .to_string(),
                ts_vector_field_metadata_universal_identifier: search_field
                    .ts_vector_field_universal_identifier
                    .to_string(),
                position: search_field.position,
                is_system_side_effect: true,
                ..UniversalFlatSearchFieldMetadata::default()
            },
        ))),
        ..UniversalCreateAction::default()
    })
}

fn index_create_action(
    index: &definitions::StandardIndexDefinition,
) -> Result<UniversalAction, SeedError> {
    let columns = index
        .columns
        .iter()
        .map(|column| UniversalFlatIndexFieldMetadata {
            order: column.order,
            sub_field_name: column.sub_field_name.map(String::from),
            field_metadata_universal_identifier: column.field_universal_identifier.to_string(),
            ..UniversalFlatIndexFieldMetadata::default()
        })
        .collect();

    Ok(UniversalAction::Create(UniversalCreateAction {
        metadata_name: MetadataName::Index,
        flat_entity: Some(UniversalFlatEntity::IndexMetadata(Box::new(
            UniversalFlatIndexMetadata {
                universal_identifier: index.universal_identifier.to_string(),
                application_universal_identifier: STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER
                    .to_string(),
                object_metadata_universal_identifier: index.object_universal_identifier.to_string(),
                name: index.name.to_string(),
                is_unique: index.is_unique,
                index_where_clause: index.index_where_clause.map(String::from),
                index_type: parse_index_type(index.index_type)?,
                is_system_side_effect: true,
                universal_flat_index_field_metadatas: columns,
                ..UniversalFlatIndexMetadata::default()
            },
        ))),
        ..UniversalCreateAction::default()
    }))
}

fn is_ts_vector_field(field: &definitions::StandardFieldDefinition) -> bool {
    field.field_type == "TS_VECTOR"
}

pub fn build_standard_seed_migration() -> Result<WorkspaceMigration, SeedError> {
    let mut actions: Vec<UniversalAction> = Vec::new();

    let mut object_ids: std::collections::HashMap<&str, Uuid> = std::collections::HashMap::new();

    for object in definitions::STANDARD_OBJECTS {
        object_ids.insert(object.universal_identifier, Uuid::new_v4());
    }

    let mut field_ids: std::collections::HashMap<&str, Uuid> = std::collections::HashMap::new();

    for object in definitions::STANDARD_OBJECTS {
        for field in object.fields {
            field_ids.insert(field.universal_identifier, Uuid::new_v4());
        }
    }

    let mut search_field_ids: std::collections::HashMap<&str, Uuid> =
        std::collections::HashMap::new();

    for search_field in definitions::STANDARD_SEARCH_FIELDS {
        search_field_ids.insert(search_field.universal_identifier, Uuid::new_v4());
    }

    for search_field in definitions::STANDARD_SEARCH_FIELDS {
        actions.push(search_field_create_action(
            search_field,
            search_field_ids[search_field.universal_identifier],
        ));
    }

    for object in definitions::STANDARD_OBJECTS {
        let mut object_fields: Vec<UniversalFlatFieldMetadata> = Vec::new();
        let mut field_id_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for field in object.fields {
            if is_relation_field(field) {
                continue;
            }

            let built = field_from_definition(field)?;

            field_id_map.insert(
                field.universal_identifier.to_string(),
                field_ids[field.universal_identifier].to_string(),
            );

            object_fields.push(built);
        }

        actions.push(object_create_action(
            object,
            object_fields,
            object_ids[object.universal_identifier],
            field_id_map,
        ));
    }

    for object in definitions::STANDARD_OBJECTS {
        for field in object.fields {
            if !is_relation_field(field) {
                continue;
            }

            actions.push(field_create_action(
                field_from_definition(field)?,
                object.universal_identifier,
                field_ids[field.universal_identifier],
            ));
        }
    }

    for option in definitions::STANDARD_FIELD_OPTIONS {
        actions.push(option_create_action(option));
    }

    for index in definitions::STANDARD_INDEXES {
        actions.push(index_create_action(index)?);
    }

    Ok(WorkspaceMigration {
        actions,
        application_universal_identifier: STANDARD_APPLICATION_UNIVERSAL_IDENTIFIER.to_string(),
    })
}

pub async fn seed_standard_objects(
    state: &AppState,
    workspace_id: Uuid,
) -> Result<SeedOutcome, SeedError> {
    if is_seeded(state, workspace_id).await? {
        return Ok(SeedOutcome::default());
    }

    ensure_standard_application(state, workspace_id).await?;

    let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);

    let mut transaction = state
        .core_pool
        .begin()
        .await
        .map_err(|error| SeedError::new("STANDARD_OBJECT_SEED_FAILED", error.to_string()))?;

    permissions::create_permission_tables(&mut transaction, &schema_name)
        .await
        .map_err(|error| SeedError::new("STANDARD_OBJECT_SEED_FAILED", error.message))?;

    transaction
        .commit()
        .await
        .map_err(|error| SeedError::new("STANDARD_OBJECT_SEED_FAILED", error.to_string()))?;

    let migration = build_standard_seed_migration()?;

    let runner = run_workspace_migration(
        state,
        RunnerInput {
            workspace_id,
            migration: &migration,
            should_defer_actions: false,
        },
    )
    .await
    .map_err(SeedError::runner)?;

    Ok(SeedOutcome {
        seeded: true,
        objects: definitions::STANDARD_OBJECTS.len(),
        fields: definitions::STANDARD_OBJECTS
            .iter()
            .map(|object| object.fields.len())
            .sum(),
        indexes: definitions::STANDARD_INDEXES.len(),
        runner: Some(runner),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nudge_core::NodeEnv;

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

    async fn test_state(url: &str) -> Option<AppState> {
        let state = AppState::connect(&test_config(url.to_string()))
            .await
            .expect("connecting nudge state");

        crate::workspace_migrations::ddl::test_support::migrate_core(url)
            .await
            .expect("running core migrations");

        Some(state)
    }

    #[test]
    fn the_definitions_match_the_reference_counts() {
        assert_eq!(definitions::STANDARD_OBJECTS.len(), 37);

        let field_count: usize = definitions::STANDARD_OBJECTS
            .iter()
            .map(|object| object.fields.len())
            .sum();

        assert_eq!(field_count, 636);
        assert_eq!(definitions::STANDARD_INDEXES.len(), 123);
        assert_eq!(definitions::STANDARD_SEARCH_FIELDS.len(), 34);
    }

    #[test]
    fn every_object_has_a_label_identifier_field() {
        for object in definitions::STANDARD_OBJECTS {
            assert!(
                object.label_identifier_field_universal_identifier.is_some(),
                "{} has no label identifier",
                object.name_singular
            );

            let identifier = object
                .label_identifier_field_universal_identifier
                .expect("label identifier");

            assert!(
                object
                    .fields
                    .iter()
                    .any(|field| field.universal_identifier == identifier),
                "{} label identifier does not resolve",
                object.name_singular
            );
        }
    }

    #[test]
    fn every_column_plan_matches_the_upstream_access_pair_layout() {
        let historical = [
            "agentChatThread",
            "agentMessage",
            "agentMessagePart",
            "agentTurn",
            "campaignDelivery",
            "messageSuppression",
        ];

        let mut historical_seen = 0usize;

        for object in definitions::STANDARD_OBJECTS {
            let plan = object.column_plan;

            assert_eq!(
                plan.first().copied(),
                Some("id"),
                "{} plan must start with id",
                object.name_singular
            );

            let access_at = plan.iter().position(|name| *name == "_accessGroupIds");
            let private_at = plan.iter().position(|name| *name == "_isPrivate");

            assert_eq!(
                access_at.map(|index| index + 1),
                private_at,
                "{} access pair must be adjacent",
                object.name_singular
            );

            let relation_columns: std::collections::HashSet<String> = object
                .fields
                .iter()
                .filter_map(relation_join_column)
                .collect();

            let non_relation: Vec<&str> = plan
                .iter()
                .copied()
                .filter(|name| !relation_columns.contains(*name))
                .collect();

            let tail = &non_relation[non_relation.len().saturating_sub(2)..];
            assert_eq!(
                tail,
                &["_accessGroupIds", "_isPrivate"],
                "{} access pair must close the CREATE TABLE portion",
                object.name_singular
            );

            let created_at = plan
                .iter()
                .position(|name| *name == "createdAt")
                .expect("createdAt in plan");

            if historical.contains(&object.name_singular) {
                historical_seen += 1;
                assert!(
                    created_at > 3,
                    "{} carries the historical base-column layout",
                    object.name_singular
                );
            } else {
                assert_eq!(
                    created_at, 1,
                    "{} uses the modern head layout",
                    object.name_singular
                );
            }
        }

        assert_eq!(historical_seen, historical.len());
    }

    fn relation_join_column(field: &definitions::StandardFieldDefinition) -> Option<String> {
        match field.field_type {
            "RELATION" => {
                let settings = parse_json(field.settings_json).ok().flatten()?;
                if settings.get("relationType").and_then(Value::as_str) != Some("MANY_TO_ONE") {
                    return None;
                }
                Some(join_column_name(&settings, field.name))
            }
            "MORPH_RELATION" => {
                if field.morph_id == Some(field.universal_identifier) {
                    return None;
                }
                let settings = parse_json(field.settings_json).ok().flatten();
                Some(match settings.as_ref() {
                    Some(value) => join_column_name(value, field.name),
                    None => format!("{}Id", field.name),
                })
            }
            _ => None,
        }
    }

    fn join_column_name(settings: &Value, field_name: &str) -> String {
        settings
            .get("joinColumnName")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| format!("{}Id", field_name))
    }

    #[test]
    fn the_seed_migration_orders_search_fields_first_and_indexes_last() {
        let migration = build_standard_seed_migration().expect("building the seed migration");

        let search_vector_universal_identifiers: std::collections::HashSet<String> =
            definitions::STANDARD_OBJECTS
                .iter()
                .flat_map(|object| object.fields.iter())
                .filter(|field| is_ts_vector_field(field))
                .map(|field| field.universal_identifier.to_string())
                .collect();

        assert!(!search_vector_universal_identifiers.is_empty());

        let mut search_field_positions: Vec<usize> = Vec::new();
        let mut object_positions: Vec<usize> = Vec::new();
        let mut field_positions: Vec<usize> = Vec::new();
        let mut option_positions: Vec<usize> = Vec::new();
        let mut index_positions: Vec<usize> = Vec::new();
        let mut update_positions: Vec<usize> = Vec::new();

        for (position, action) in migration.actions.iter().enumerate() {
            match action {
                UniversalAction::Create(create) => match create.flat_entity.as_ref() {
                    Some(UniversalFlatEntity::SearchFieldMetadata(search_field)) => {
                        assert!(
                            search_vector_universal_identifiers.contains(
                                &search_field.ts_vector_field_metadata_universal_identifier
                            ),
                            "search field references an unknown ts vector field"
                        );
                        assert!(create.id.is_some(), "search field rows carry ids");
                        search_field_positions.push(position);
                    }
                    Some(UniversalFlatEntity::ObjectMetadata(object)) => {
                        assert!(create.id.is_some(), "object rows carry ids");

                        let definition = definitions::STANDARD_OBJECTS
                            .iter()
                            .find(|candidate| {
                                candidate.universal_identifier == object.universal_identifier
                            })
                            .expect("seeded object exists in the definitions");

                        let expected: std::collections::HashSet<&str> = definition
                            .fields
                            .iter()
                            .filter(|field| !is_relation_field(field))
                            .map(|field| field.universal_identifier)
                            .collect();

                        let embedded: std::collections::HashSet<&str> = create
                            .universal_flat_field_metadatas
                            .iter()
                            .map(|field| field.universal_identifier.as_str())
                            .collect();

                        assert_eq!(
                            embedded, expected,
                            "{} embeds the wrong field set",
                            definition.name_singular
                        );

                        for field in &create.universal_flat_field_metadatas {
                            assert!(
                                !matches!(
                                    field.field_type,
                                    FieldMetadataType::Relation | FieldMetadataType::MorphRelation
                                ),
                                "{} carries a relation field inline",
                                object.name_singular
                            );
                        }

                        object_positions.push(position);
                    }
                    Some(UniversalFlatEntity::FieldMetadata(field)) => {
                        assert!(
                            matches!(
                                field.field_type,
                                FieldMetadataType::Relation | FieldMetadataType::MorphRelation
                            ),
                            "non relation field {} arrived as a standalone action",
                            field.name
                        );
                        assert!(create.id.is_some(), "field rows carry ids");
                        field_positions.push(position);
                    }
                    Some(UniversalFlatEntity::FieldMetadataOption(_)) => {
                        option_positions.push(position);
                    }
                    Some(UniversalFlatEntity::IndexMetadata(_)) => {
                        index_positions.push(position);
                    }
                    _ => {}
                },
                UniversalAction::Update(update) => {
                    assert_ne!(update.rebuild_search_vector, Some(true));
                    update_positions.push(position);
                }
                _ => {}
            }
        }

        assert_eq!(
            search_field_positions.len(),
            definitions::STANDARD_SEARCH_FIELDS.len()
        );
        assert_eq!(object_positions.len(), definitions::STANDARD_OBJECTS.len());
        assert_eq!(
            option_positions.len(),
            definitions::STANDARD_FIELD_OPTIONS.len()
        );
        assert_eq!(index_positions.len(), definitions::STANDARD_INDEXES.len());
        assert!(update_positions.is_empty());

        let relation_field_count: usize = definitions::STANDARD_OBJECTS
            .iter()
            .flat_map(|object| object.fields.iter())
            .filter(|field| is_relation_field(field))
            .count();

        assert_eq!(field_positions.len(), relation_field_count);

        let last_search_field = *search_field_positions.last().expect("search fields");
        let first_object = *object_positions.first().expect("objects");
        let last_object = *object_positions.last().expect("objects");
        let first_relation = *field_positions.first().expect("relation fields");
        let last_relation = *field_positions.last().expect("relation fields");
        let first_option = *option_positions.first().expect("options");
        let first_index = *index_positions.first().expect("indexes");

        assert!(
            last_search_field < first_object,
            "search field rows must be inserted before object rows"
        );
        assert!(
            last_object < first_relation,
            "relation field actions must run after every object create"
        );
        assert!(
            last_relation < first_option,
            "option actions must run after relation fields exist"
        );
        assert!(
            first_option < first_index,
            "index actions must run after options exist"
        );
    }

    #[test]
    fn every_index_resolves_its_object_and_fields() {
        let object_identifiers: std::collections::HashSet<&str> = definitions::STANDARD_OBJECTS
            .iter()
            .map(|object| object.universal_identifier)
            .collect();

        let field_identifiers: std::collections::HashSet<&str> = definitions::STANDARD_OBJECTS
            .iter()
            .flat_map(|object| object.fields.iter())
            .map(|field| field.universal_identifier)
            .collect();

        for index in definitions::STANDARD_INDEXES {
            assert!(
                object_identifiers.contains(index.object_universal_identifier),
                "index {} has an unknown object",
                index.name
            );

            for column in index.columns {
                assert!(
                    field_identifiers.contains(column.field_universal_identifier),
                    "index {} has an unknown field",
                    index.name
                );
            }
        }
    }

    #[test]
    fn every_relation_target_resolves() {
        let object_identifiers: std::collections::HashSet<&str> = definitions::STANDARD_OBJECTS
            .iter()
            .map(|object| object.universal_identifier)
            .collect();

        let field_identifiers: std::collections::HashSet<&str> = definitions::STANDARD_OBJECTS
            .iter()
            .flat_map(|object| object.fields.iter())
            .map(|field| field.universal_identifier)
            .collect();

        for object in definitions::STANDARD_OBJECTS {
            for field in object.fields {
                if let Some(target) = field.relation_target_object_universal_identifier {
                    assert!(
                        object_identifiers.contains(target),
                        "{}.{} has an unknown relation target object",
                        object.name_singular,
                        field.name
                    );
                }

                if let Some(target) = field.relation_target_field_universal_identifier {
                    assert!(
                        field_identifiers.contains(target),
                        "{}.{} has an unknown relation target field",
                        object.name_singular,
                        field.name
                    );
                }
            }
        }
    }

    fn strip_family_hex(value: &str, family: &str) -> String {
        let mut out = String::with_capacity(value.len());
        let mut rest = value;

        while let Some(position) = rest.find(family) {
            out.push_str(&rest[..position + family.len()]);

            let tail = &rest[position + family.len()..];
            let hex_len = tail.chars().take_while(|ch| ch.is_ascii_hexdigit()).count();

            rest = &tail[hex_len..];
        }

        out.push_str(rest);
        out
    }

    fn normalize_schema_line(line: &str, schema: &str) -> String {
        let without_schema = line
            .replace(&format!("\"{schema}\""), "ws")
            .replace(&format!("{schema}."), "ws.")
            .replace(schema, "ws");

        let without_access_group = strip_family_hex(&without_schema, "IDX_ACCESS_GROUP_IDS_");

        strip_family_hex(&without_access_group, "IDX_PRIVATE_RECORD_")
    }

    type ColumnDumpRow = (
        String,
        String,
        i32,
        String,
        bool,
        String,
        String,
        Option<String>,
    );

    async fn dump_normalized_schema(url: &str, schema: &str) -> Vec<String> {
        let pool = sqlx::PgPool::connect(url)
            .await
            .expect("connecting for the schema dump");

        let columns: Vec<ColumnDumpRow> = sqlx::query_as(
            "SELECT c.relname::text, a.attname::text, a.attnum::int, \
                format_type(a.atttypid, a.atttypmod), a.attnotnull, a.attidentity::text, \
                a.attgenerated::text, pg_get_expr(d.adbin, d.adrelid) \
             FROM pg_attribute a \
             JOIN pg_class c ON c.oid = a.attrelid \
             JOIN pg_namespace n ON n.oid = c.relnamespace \
             LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
             WHERE n.nspname = $1 AND c.relkind = 'r' AND a.attnum > 0 \
               AND NOT a.attisdropped \
             ORDER BY c.relname, a.attnum",
        )
        .bind(schema)
        .fetch_all(&pool)
        .await
        .expect("dumping columns");

        let mut columns_by_table: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();

        for (table, name, attnum, type_name, not_null, identity, generated, default) in columns {
            let line = format!(
                "column {table}.{name} attnum={attnum} type={type_name} notnull={not_null} identity={identity} generated={generated} default={default:?}"
            );
            columns_by_table.entry(table).or_default().push(line);
        }

        let mut index_lines: Vec<String> = Vec::new();
        let mut constraint_lines: Vec<String> = Vec::new();
        let mut enum_lines: Vec<String> = Vec::new();

        let indexes: Vec<(String, String)> = sqlx::query_as(
            "SELECT indexname::text, indexdef FROM pg_indexes WHERE schemaname = $1",
        )
        .bind(schema)
        .fetch_all(&pool)
        .await
        .expect("dumping indexes");

        for (name, definition) in indexes {
            index_lines.push(format!("index {name} def={definition}"));
        }

        let constraints: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT c.conname::text, c.contype::text, rel.relname::text, \
                pg_get_constraintdef(c.oid) \
             FROM pg_constraint c \
             JOIN pg_class rel ON rel.oid = c.conrelid \
             JOIN pg_namespace n ON n.oid = rel.relnamespace \
             WHERE n.nspname = $1",
        )
        .bind(schema)
        .fetch_all(&pool)
        .await
        .expect("dumping constraints");

        for (name, kind, table, definition) in constraints {
            constraint_lines.push(format!(
                "constraint {name} kind={kind} table={table} def={definition}"
            ));
        }

        let enums: Vec<(String, String)> = sqlx::query_as(
            "SELECT t.typname::text, \
                string_agg(e.enumlabel::text, ',' ORDER BY e.enumsortorder) \
             FROM pg_type t \
             JOIN pg_enum e ON e.enumtypid = t.oid \
             JOIN pg_namespace n ON n.oid = t.typnamespace \
             WHERE n.nspname = $1 \
             GROUP BY t.typname",
        )
        .bind(schema)
        .fetch_all(&pool)
        .await
        .expect("dumping enums");

        for (name, labels) in enums {
            enum_lines.push(format!("enum {name} values={labels}"));
        }

        let mut column_part: Vec<String> = Vec::new();

        for (_table, lines) in columns_by_table {
            column_part.extend(lines);
        }

        let mut tail_part: Vec<String> = Vec::new();
        tail_part.extend(index_lines);
        tail_part.extend(constraint_lines);
        tail_part.extend(enum_lines);

        let mut tail_lines: Vec<String> = tail_part
            .iter()
            .map(|line| normalize_schema_line(line, schema))
            .collect();
        tail_lines.sort();

        let mut normalized_columns: Vec<String> = column_part
            .iter()
            .map(|line| normalize_schema_line(line, schema))
            .collect();

        normalized_columns.extend(tail_lines);
        normalized_columns
    }

    fn group_column_lines(dump: &[String]) -> std::collections::BTreeMap<&str, Vec<&String>> {
        let mut grouped: std::collections::BTreeMap<&str, Vec<&String>> =
            std::collections::BTreeMap::new();

        for line in dump {
            if let Some(rest) = line.strip_prefix("column ") {
                let table = rest.split('.').next().unwrap_or(rest);
                grouped.entry(table).or_default().push(line);
            }
        }

        grouped
    }

    fn non_column_lines(dump: &[String]) -> Vec<&String> {
        dump.iter()
            .filter(|line| !line.starts_with("column "))
            .collect()
    }

    fn membership_key(line: &str) -> String {
        match line.strip_prefix("column ") {
            Some(rest) => {
                let without_attnum: Vec<&str> = rest
                    .split_whitespace()
                    .filter(|token| !token.starts_with("attnum="))
                    .collect();
                format!("column {}", without_attnum.join(" "))
            }
            None => line.to_string(),
        }
    }

    #[tokio::test]
    async fn the_seeded_workspace_schema_matches_the_reference() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };

        let Ok(reference_url) = std::env::var("NUDGE_REFERENCE_DATABASE_URL") else {
            return;
        };

        let _guard = TEST_LOCK.lock().await;

        let state = test_state(&url).await.expect("connecting the test state");

        let workspace_id = Uuid::new_v4();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let application_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let mut fixture = state.core_pool.begin().await.expect("fixture tx");

        sqlx::query(
            "INSERT INTO core.\"workspace\" \
             (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
              \"workspaceCustomApplicationId\", \"isPasswordAuthEnabled\", \"databaseSchema\") \
             VALUES ($1, 'Seed Parity', $2, 'PENDING_CREATION', $3, $4, true, $5)",
        )
        .bind(workspace_id)
        .bind(Uuid::new_v4())
        .bind(format!(
            "parity-{}",
            &workspace_id.simple().to_string()[..8]
        ))
        .bind(application_id)
        .bind(&schema_name)
        .execute(&mut *fixture)
        .await
        .expect("inserting the parity workspace");

        sqlx::query(
            "INSERT INTO core.\"application\" \
             (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", \"workspaceId\") \
             VALUES ($1, $2, 'Seed Parity Application', 'LOCAL', 'INSTALLED', '', $3)",
        )
        .bind(application_id)
        .bind(Uuid::new_v4())
        .bind(workspace_id)
        .execute(&mut *fixture)
        .await
        .expect("inserting the custom application");

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
        .expect("activating the parity workspace");

        fixture.commit().await.expect("committing the fixture");

        sqlx::query(&format!("CREATE SCHEMA \"{schema_name}\""))
            .execute(&state.core_pool)
            .await
            .expect("creating the parity schema");

        let outcome = seed_standard_objects(&state, workspace_id)
            .await
            .expect("seeding the standard objects");

        assert!(outcome.seeded);

        let reference_pool = sqlx::PgPool::connect(&reference_url)
            .await
            .expect("connecting the reference database");

        let reference_schema: (String,) = sqlx::query_as(
            "SELECT schema_name FROM information_schema.schemata \
             WHERE schema_name LIKE 'workspace_%' LIMIT 1",
        )
        .fetch_one(&reference_pool)
        .await
        .expect("the reference database has a workspace schema");

        let mine = dump_normalized_schema(&url, &schema_name).await;
        let theirs = dump_normalized_schema(&reference_url, &reference_schema.0).await;

        sqlx::query(&format!("DROP SCHEMA \"{schema_name}\" CASCADE"))
            .execute(&state.core_pool)
            .await
            .expect("dropping the parity schema");

        assert!(!mine.is_empty() && !theirs.is_empty());

        if mine != theirs {
            let my_columns = group_column_lines(&mine);
            let their_columns = group_column_lines(&theirs);

            let mut divergent_tables = 0usize;
            let mut divergences: Vec<String> = Vec::new();

            let table_names: std::collections::BTreeSet<&str> = my_columns
                .keys()
                .chain(their_columns.keys())
                .copied()
                .collect();

            for table in table_names {
                let my_lines = my_columns.get(table);
                let their_lines = their_columns.get(table);

                if my_lines == their_lines {
                    continue;
                }

                divergent_tables += 1;

                let my_vec = my_lines.cloned().unwrap_or_default();
                let their_vec = their_lines.cloned().unwrap_or_default();

                let position = my_vec
                    .iter()
                    .zip(their_vec.iter())
                    .position(|(a, b)| a != b)
                    .unwrap_or_else(|| my_vec.len().min(their_vec.len()));

                let mine_at = my_vec
                    .get(position)
                    .map(|line| line.as_str())
                    .unwrap_or("<none>");
                let theirs_at = their_vec
                    .get(position)
                    .map(|line| line.as_str())
                    .unwrap_or("<none>");

                divergences.push(format!(
                    "  {table} first divergence at position {position} ({} columns vs {}):\n    mine:   {mine_at}\n    theirs: {theirs_at}\n",
                    my_vec.len(),
                    their_vec.len()
                ));
            }

            let set_mine: std::collections::HashSet<&String> =
                non_column_lines(&mine).into_iter().collect();
            let set_theirs: std::collections::HashSet<&String> =
                non_column_lines(&theirs).into_iter().collect();

            let mut extra: Vec<&&String> = set_mine.difference(&set_theirs).collect();
            let mut missing: Vec<&&String> = set_theirs.difference(&set_mine).collect();
            extra.sort();
            missing.sort();

            let membership_mine: std::collections::HashSet<String> =
                mine.iter().map(|line| membership_key(line)).collect();
            let membership_theirs: std::collections::HashSet<String> =
                theirs.iter().map(|line| membership_key(line)).collect();

            let mut extra_members: Vec<&String> =
                membership_mine.difference(&membership_theirs).collect();
            let mut missing_members: Vec<&String> =
                membership_theirs.difference(&membership_mine).collect();
            extra_members.sort();
            missing_members.sort();

            let mut report = format!(
                "workspace schema parity: ordered column vectors differ for {divergent_tables} tables\n"
            );
            report.push_str(&format!(
                "membership set diff (attnum ignored): {} extra, {} missing\n",
                extra_members.len(),
                missing_members.len()
            ));

            for line in extra_members.iter().take(30) {
                report.push_str(&format!("  + {line}\n"));
            }
            for line in missing_members.iter().take(30) {
                report.push_str(&format!("  - {line}\n"));
            }

            for divergence in &divergences {
                report.push_str(divergence);
            }

            if !extra.is_empty() || !missing.is_empty() {
                report.push_str(&format!(
                    "non-column sections: {} extra, {} missing\n",
                    extra.len(),
                    missing.len()
                ));
                for line in extra.iter().take(30) {
                    report.push_str(&format!("  + {line}\n"));
                }
                for line in missing.iter().take(30) {
                    report.push_str(&format!("  - {line}\n"));
                }
            }

            panic!("{report}");
        }
    }
}
