use std::collections::HashMap;
use std::collections::HashSet;

use icu_collator::options::CollatorOptions;
use icu_collator::Collator;
use icu_collator::CollatorPreferences;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;
use sqlx::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::metadata::flat::FlatObjectMetadata;
use crate::metadata::flat::FlatWorkspaceMaps;
use crate::metadata::names::compute_composite_column_name;
use crate::permissions::error::PermissionError;
use crate::permissions::error::PermissionErrorCode;
use crate::permissions::model::ObjectRecordsOperationType;
use crate::permissions::model::ObjectRecordsPermissions;
use crate::permissions::model::PermissionPrincipalKind;
use crate::permissions::model::RoleRecordsPermissions;
use crate::permissions::model::RowLevelRecordsPermissions;
use crate::query::filter::Suffixes;
use crate::query::repository::quote_identifier;
use crate::query::runner::RunnerSchema;
use crate::state::AppState;

const ACCESS_GROUP_IDS_COLUMN_NAME: &str = "_accessGroupIds";
const ACCESS_GROUP_GENERATION_COLUMN_NAME: &str = "_accessGroupGeneration";
const ACCESS_GROUP_MATERIALIZED_GENERATION_COLUMN_NAME: &str = "materializedGeneration";
const ACCESS_GROUP_DIRTY_INDEX_NAME: &str = "IDX_COMPANY_ACCESS_GROUP_DIRTY";
const ACCESS_GROUP_STATE_TABLE_NAME: &str = "accessGroupState";
const ACCESS_GROUP_REBUILD_SETTING: &str = "nudge.access_group_rebuild";
const COMPANY_OBJECT_NAME: &str = "company";
const WORKSPACE_MEMBER_OBJECT_NAME: &str = "workspaceMember";
const ACCESS_GROUP_FEATURE_FLAGS: [&str; 4] = [
    "IS_ACCESS_GROUPS_ENABLED",
    "IS_ROW_LEVEL_PERMISSION_RELATION_HOP_ENABLED",
    "IS_PRIVATE_RECORDS_ENABLED",
    "IS_RECORD_LEVEL_SHARING_ENABLED",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompanyAccessStrategy {
    pub workspace_id: Uuid,
    pub company_object_id: Uuid,
    pub principal_kind: PermissionPrincipalKind,
    pub workspace_member_id: Option<Uuid>,
    pub role_permissions: Vec<ObjectRecordsPermissions>,
    pub enabled: bool,
    pub relation_hop_enabled: bool,
    pub overlays_enabled: bool,
    pub limiting_roles: bool,
}

impl CompanyAccessStrategy {
    pub fn denies_overlays(&self) -> bool {
        self.overlays_enabled && self.principal_kind != PermissionPrincipalKind::System
    }

    pub fn can_fast_path_read(&self) -> bool {
        self.enabled
            && self.principal_kind == PermissionPrincipalKind::WorkspaceMember
            && self.workspace_member_id.is_some()
            && !self.limiting_roles
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AccessGroupType {
    AllRecords,
    RlsRule,
    RlsMemberValue,
    RlsLiteralValue,
}

impl AccessGroupType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllRecords => "ALL_RECORDS",
            Self::RlsRule => "RLS_RULE",
            Self::RlsMemberValue => "RLS_MEMBER_VALUE",
            Self::RlsLiteralValue => "RLS_LITERAL_VALUE",
        }
    }

    fn parse(value: &str) -> Result<Self, AccessGroupError> {
        match value {
            "ALL_RECORDS" => Ok(Self::AllRecords),
            "RLS_RULE" => Ok(Self::RlsRule),
            "RLS_MEMBER_VALUE" => Ok(Self::RlsMemberValue),
            "RLS_LITERAL_VALUE" => Ok(Self::RlsLiteralValue),
            _ => Err(AccessGroupError::invalid(format!(
                "Unknown access group type {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccessGroupWithoutCompactId {
    pub object_metadata_id: Uuid,
    pub group_type: AccessGroupType,
    pub rule_shape_hash: String,
    pub unreadable_field_ids: Vec<Uuid>,
    pub member_field_values: Option<Value>,
    pub value: Option<String>,
    pub group_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccessGroup {
    pub compact_id: i32,
    pub group: AccessGroupWithoutCompactId,
}

#[derive(Debug, Clone)]
struct GroupPlan {
    group: AccessGroupWithoutCompactId,
    rule: Option<RowLevelRecordsPermissions>,
}

#[derive(Debug)]
pub(crate) enum AccessGroupError {
    Database(sqlx::Error),
    Invalid(String),
    Unsupported(String),
}

impl AccessGroupError {
    fn invalid(message: impl Into<String>) -> Self {
        Self::Invalid(message.into())
    }

    fn unsupported(message: impl Into<String>) -> Self {
        Self::Unsupported(message.into())
    }
}

impl std::fmt::Display for AccessGroupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "ACCESS_GROUP_QUERY_FAILED: {error}"),
            Self::Invalid(message) => write!(formatter, "ACCESS_GROUP_INVALID: {message}"),
            Self::Unsupported(message) => write!(formatter, "ACCESS_GROUP_UNSUPPORTED: {message}"),
        }
    }
}

impl std::error::Error for AccessGroupError {}

impl From<sqlx::Error> for AccessGroupError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<crate::metadata::error::MetadataError> for AccessGroupError {
    fn from(error: crate::metadata::error::MetadataError) -> Self {
        Self::Database(sqlx::Error::Protocol(error.to_string()))
    }
}

impl From<crate::query::repository::RepositoryError> for AccessGroupError {
    fn from(error: crate::query::repository::RepositoryError) -> Self {
        Self::Invalid(error.to_string())
    }
}

impl From<AccessGroupError> for PermissionError {
    fn from(error: AccessGroupError) -> Self {
        PermissionError::new(
            PermissionErrorCode::RecordPermissionQueryFailed,
            error.to_string(),
        )
    }
}

#[derive(Debug, Default)]
pub(crate) struct MaterializationOutcome {
    pub group_count: usize,
    pub updated_record_count: usize,
}

#[derive(Debug)]
pub(crate) enum RebuildOutcome {
    Disabled,
    NoCompany,
    Unsupported,
    Materialized(MaterializationOutcome),
}

#[derive(Debug)]
struct RuleShape {
    rule_shape_hash: String,
    unreadable_field_ids: Vec<Uuid>,
    split_predicate: Option<Value>,
}

#[derive(Debug, Clone)]
enum SplitValue {
    AllRecords,
    RlsRule,
    Member(Map<String, Value>),
    Literal(String),
}

pub(crate) async fn prepare_company_read(
    state: &AppState,
    maps: &FlatWorkspaceMaps,
    permissions: &mut RoleRecordsPermissions,
) -> Result<(), PermissionError> {
    let Some(strategy) = permissions.company_access.clone() else {
        return Ok(());
    };
    if strategy.denies_overlays() || !strategy.can_fast_path_read() {
        return Ok(());
    }
    let Some(object) = permissions.object(strategy.company_object_id) else {
        return Ok(());
    };
    if object.can_read_all_records {
        return Ok(());
    }
    let runner = build_runner_schema(&strategy.workspace_id, maps)?;
    if !read_rules_supported(&runner, COMPANY_OBJECT_NAME, object) {
        return Ok(());
    }
    let predicates = member_predicates(object);
    let member_values = if let Some(member_id) = strategy.workspace_member_id {
        if predicates.is_empty() {
            None
        } else {
            Some({
                let mut connection = state
                    .core_pool
                    .acquire()
                    .await
                    .map_err(AccessGroupError::from)?;
                load_member_field_values(&mut connection, &runner, member_id, &predicates)
                    .await
                    .map_err(PermissionError::from)?
            })
        }
    } else {
        None
    };
    let groups = build_role_access_groups(
        strategy.company_object_id,
        object,
        ObjectRecordsOperationType::Read,
        member_values.as_ref(),
        strategy.relation_hop_enabled,
    )?;
    let mut compact_ids = find_materialized_compact_ids(
        &state.core_pool,
        &runner,
        strategy.workspace_id,
        strategy.company_object_id,
        &groups,
    )
    .await?;
    if compact_ids.is_none() {
        match rebuild_company_access_groups(&state.core_pool, strategy.workspace_id).await {
            Ok(RebuildOutcome::Materialized(_)) | Ok(RebuildOutcome::NoCompany) => {}
            Ok(RebuildOutcome::Unsupported) | Ok(RebuildOutcome::Disabled) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
        compact_ids = find_materialized_compact_ids(
            &state.core_pool,
            &runner,
            strategy.workspace_id,
            strategy.company_object_id,
            &groups,
        )
        .await?;
    }
    permissions.company_access_group_ids = compact_ids;
    Ok(())
}

pub(crate) async fn rebuild_company_access_groups(
    pool: &PgPool,
    workspace_id: Uuid,
) -> Result<RebuildOutcome, AccessGroupError> {
    let mut invalidation = pool.begin().await?;
    lock_workspace_access_groups(&mut invalidation, workspace_id).await?;
    let maps = crate::metadata::flat::load_workspace_flat_maps_with_executor(
        &mut invalidation,
        workspace_id,
    )
    .await?;
    if maps
        .object_metadata_maps
        .by_universal_identifier
        .values()
        .any(|object| object.name_singular == COMPANY_OBJECT_NAME && object.is_active)
    {
        let runner = build_runner_schema(&workspace_id, &maps)?;
        ensure_company_access_storage(&mut invalidation, &runner).await?;
        bump_access_group_generation(&mut invalidation, &runner).await?;
        invalidate_company_arrays(&mut invalidation, &runner, None).await?;
    }
    invalidation.commit().await?;

    let mut transaction = pool.begin().await?;
    let outcome =
        rebuild_company_access_groups_inner(&mut transaction, pool, workspace_id, None).await?;
    transaction.commit().await?;
    Ok(outcome)
}

pub(crate) async fn lock_company_access_groups(
    transaction: &mut PgConnection,
    schema: &RunnerSchema,
) -> Result<(), AccessGroupError> {
    let Some(strategy) = schema.company_access() else {
        return Ok(());
    };
    lock_workspace_access_groups(transaction, strategy.workspace_id).await?;
    ensure_company_access_storage(transaction, schema).await
}

async fn lock_workspace_access_groups(
    transaction: &mut PgConnection,
    workspace_id: Uuid,
) -> Result<(), AccessGroupError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(format!("nudge-access-groups:{workspace_id}"))
        .execute(&mut *transaction)
        .await?;
    Ok(())
}

pub(crate) async fn recompute_after_write(
    pool: &PgPool,
    transaction: &mut PgConnection,
    schema: &RunnerSchema,
    object_name: &str,
    ids: &[Uuid],
) -> Result<RebuildOutcome, AccessGroupError> {
    if !schema.requires_access_group_recompute(object_name) {
        return Ok(RebuildOutcome::Disabled);
    }
    lock_company_access_groups(transaction, schema).await?;
    if object_name == "featureFlag" {
        bump_access_group_generation(transaction, schema).await?;
    }
    let workspace_id = schema
        .company_access()
        .map(|strategy| strategy.workspace_id)
        .unwrap_or_default();
    let record_ids = if object_name == COMPANY_OBJECT_NAME {
        Some(ids)
    } else {
        None
    };
    match rebuild_company_access_groups_inner(transaction, pool, workspace_id, record_ids).await {
        Ok(RebuildOutcome::Unsupported) | Ok(RebuildOutcome::Disabled) => {
            invalidate_company_arrays(transaction, schema, record_ids).await?;
            Ok(RebuildOutcome::Unsupported)
        }
        outcome => outcome,
    }
}

async fn rebuild_company_access_groups_inner(
    transaction: &mut PgConnection,
    pool: &PgPool,
    workspace_id: Uuid,
    record_ids: Option<&[Uuid]>,
) -> Result<RebuildOutcome, AccessGroupError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(format!("nudge-access-groups:{workspace_id}"))
        .execute(&mut *transaction)
        .await?;

    let maps = crate::metadata::flat::load_workspace_flat_maps_with_executor(
        &mut *transaction,
        workspace_id,
    )
    .await?;
    let company = maps
        .object_metadata_maps
        .by_universal_identifier
        .values()
        .find(|object| object.name_singular == COMPANY_OBJECT_NAME && object.is_active)
        .cloned();
    let Some(company) = company else {
        return Ok(RebuildOutcome::NoCompany);
    };
    let objects = maps
        .object_metadata_maps
        .by_universal_identifier
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let fields = maps
        .field_metadata_maps
        .by_universal_identifier
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let workspace = crate::permissions::loader::load_permission_set_with_executor(
        &mut *transaction,
        workspace_id,
        &objects,
        &fields,
    )
    .await
    .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
    if !workspace
        .feature_flags
        .get("IS_ACCESS_GROUPS_ENABLED")
        .copied()
        .unwrap_or(false)
    {
        return Ok(RebuildOutcome::Disabled);
    }

    let runner = build_runner_schema(&workspace_id, &maps)?;
    ensure_company_access_storage(transaction, &runner).await?;
    let members = load_workspace_members(transaction, &runner, workspace_id).await?;
    let mut plans = Vec::<GroupPlan>::new();
    let mut seen_hashes = HashSet::new();

    for (role_id, role) in &workspace.roles {
        let Some(object) = role.object(company.id) else {
            continue;
        };
        if !read_rules_supported(&runner, COMPANY_OBJECT_NAME, object) {
            return Ok(RebuildOutcome::Unsupported);
        }
        let predicates = member_predicates(object);
        let role_members = members
            .iter()
            .filter(|(member_role, _)| member_role == role_id)
            .map(|(_, member_id)| *member_id)
            .collect::<Vec<_>>();
        let member_values = if predicates.is_empty() || role_members.is_empty() {
            Vec::new()
        } else {
            let mut values = Vec::new();
            for member_id in role_members {
                values.push(
                    load_member_field_values(transaction, &runner, member_id, &predicates).await?,
                );
            }
            values
        };
        let groups = if predicates.is_empty() {
            build_role_access_groups(
                company.id,
                object,
                ObjectRecordsOperationType::Read,
                None,
                workspace.relation_hop_enabled(),
            )?
        } else {
            let mut groups = Vec::new();
            for values in &member_values {
                groups.extend(build_role_access_groups(
                    company.id,
                    object,
                    ObjectRecordsOperationType::Read,
                    Some(values),
                    workspace.relation_hop_enabled(),
                )?);
            }
            groups
        };
        for group in groups {
            if !seen_hashes.insert(group.group_hash.clone()) {
                continue;
            }
            let rule = group_rule_for_group(object, &group)?;
            plans.push(GroupPlan { group, rule });
        }
    }

    let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
    crate::standard_objects::permissions::create_permission_tables(transaction, &schema_name)
        .await
        .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
    for plan in &plans {
        insert_access_group(transaction, &schema_name, &plan.group).await?;
    }
    let group_hashes = plans
        .iter()
        .map(|plan| plan.group.group_hash.clone())
        .collect::<Vec<_>>();
    sqlx::query(&format!(
        "DELETE FROM {}.{} WHERE {} = $1 AND NOT ({} = ANY($2::text[]))",
        quote_identifier(&schema_name)?,
        quote_identifier("accessGroup")?,
        quote_identifier("objectMetadataId")?,
        quote_identifier("groupHash")?
    ))
    .bind(company.id)
    .bind(group_hashes)
    .execute(&mut *transaction)
    .await?;

    let stored = load_company_access_groups(&mut *transaction, &schema_name, company.id).await?;
    let mut rule_shapes = HashMap::new();
    for plan in &plans {
        if let Some(rule) = &plan.rule {
            rule_shapes.insert(plan.group.rule_shape_hash.clone(), rule.clone());
        }
    }
    let updated_record_count = recompute_company_arrays(
        &mut *transaction,
        pool,
        &runner,
        &schema_name,
        &company,
        &stored,
        &rule_shapes,
        record_ids,
    )
    .await?;
    Ok(RebuildOutcome::Materialized(MaterializationOutcome {
        group_count: stored.len(),
        updated_record_count,
    }))
}

async fn ensure_company_access_storage(
    transaction: &mut PgConnection,
    schema: &RunnerSchema,
) -> Result<(), AccessGroupError> {
    let company = schema
        .object(COMPANY_OBJECT_NAME)
        .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
    let member = schema
        .object(WORKSPACE_MEMBER_OBJECT_NAME)
        .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
    let schema_name = company.shape.schema_name.as_str();
    let state_table = format!(
        "{}.{}",
        quote_identifier(schema_name)?,
        quote_identifier(ACCESS_GROUP_STATE_TABLE_NAME)?
    );
    let state_exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(&state_table)
        .fetch_one(&mut *transaction)
        .await?;
    if !state_exists {
        sqlx::query(&format!(
            "CREATE TABLE {} (id smallint PRIMARY KEY DEFAULT 1 CHECK (id = 1), generation bigint \
             NOT NULL DEFAULT 0, {} integer NOT NULL DEFAULT 0)",
            state_table,
            quote_identifier(ACCESS_GROUP_MATERIALIZED_GENERATION_COLUMN_NAME)?
        ))
        .execute(&mut *transaction)
        .await?;
        sqlx::query(&format!(
            "INSERT INTO {} (id, generation) VALUES (1, 0) ON CONFLICT DO NOTHING",
            state_table
        ))
        .execute(&mut *transaction)
        .await?;
    } else {
        let materialized_column_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
             WHERE table_schema = $1 AND table_name = $2 AND column_name = $3)",
        )
        .bind(schema_name)
        .bind(ACCESS_GROUP_STATE_TABLE_NAME)
        .bind(ACCESS_GROUP_MATERIALIZED_GENERATION_COLUMN_NAME)
        .fetch_one(&mut *transaction)
        .await?;
        if !materialized_column_exists {
            sqlx::query(&format!(
                "ALTER TABLE {} ADD COLUMN IF NOT EXISTS {} integer NOT NULL DEFAULT 0",
                state_table,
                quote_identifier(ACCESS_GROUP_MATERIALIZED_GENERATION_COLUMN_NAME)?
            ))
            .execute(&mut *transaction)
            .await?;
        }
    }
    let generation_column_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns \
         WHERE table_schema = $1 AND table_name = $2 AND column_name = $3)",
    )
    .bind(schema_name)
    .bind(&company.shape.table_name)
    .bind(ACCESS_GROUP_GENERATION_COLUMN_NAME)
    .fetch_one(&mut *transaction)
    .await?;
    if !generation_column_exists {
        sqlx::query(&format!(
            "ALTER TABLE {}.{} ADD COLUMN {} bigint",
            quote_identifier(schema_name)?,
            quote_identifier(&company.shape.table_name)?,
            quote_identifier(ACCESS_GROUP_GENERATION_COLUMN_NAME)?
        ))
        .execute(&mut *transaction)
        .await?;
    }
    let dirty_index_exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(format!(
            "{}.{}",
            quote_identifier(schema_name)?,
            quote_identifier(ACCESS_GROUP_DIRTY_INDEX_NAME)?
        ))
        .fetch_one(&mut *transaction)
        .await?;
    if !dirty_index_exists {
        sqlx::query(&format!(
            "CREATE INDEX IF NOT EXISTS {} ON {}.{} ({}) WHERE {} IS NULL OR {} IS NULL",
            quote_identifier(ACCESS_GROUP_DIRTY_INDEX_NAME)?,
            quote_identifier(schema_name)?,
            quote_identifier(&company.shape.table_name)?,
            quote_identifier("id")?,
            quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME)?,
            quote_identifier(ACCESS_GROUP_GENERATION_COLUMN_NAME)?
        ))
        .execute(&mut *transaction)
        .await?;
    }
    if !trigger_exists(
        transaction,
        schema_name,
        &company.shape.table_name,
        "nudge_invalidate_company_access_groups",
    )
    .await?
    {
        sqlx::query(&format!(
            "CREATE OR REPLACE FUNCTION {}.nudge_invalidate_company_access_groups() RETURNS trigger \
             LANGUAGE plpgsql AS $$ BEGIN \
             IF current_setting('{}', true) = 'on' THEN RETURN NEW; END IF; \
             NEW.{} := NULL; NEW.{} := NULL; RETURN NEW; END $$",
            quote_identifier(schema_name)?,
            ACCESS_GROUP_REBUILD_SETTING,
            quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME)?,
            quote_identifier(ACCESS_GROUP_GENERATION_COLUMN_NAME)?
        ))
        .execute(&mut *transaction)
        .await?;
        sqlx::query(&format!(
            "CREATE TRIGGER nudge_invalidate_company_access_groups \
             BEFORE INSERT OR UPDATE ON {}.{} FOR EACH ROW \
             EXECUTE FUNCTION {}.nudge_invalidate_company_access_groups()",
            quote_identifier(schema_name)?,
            quote_identifier(&company.shape.table_name)?,
            quote_identifier(schema_name)?
        ))
        .execute(&mut *transaction)
        .await?;
    }
    if !trigger_exists(
        transaction,
        schema_name,
        &member.shape.table_name,
        "nudge_bump_access_group_generation",
    )
    .await?
    {
        sqlx::query(&format!(
            "CREATE OR REPLACE FUNCTION {}.nudge_bump_access_group_generation() RETURNS trigger \
             LANGUAGE plpgsql AS $$ BEGIN \
             UPDATE {} SET generation = generation + 1 WHERE id = 1; \
             IF TG_OP = 'DELETE' THEN RETURN OLD; END IF; RETURN NEW; END $$",
            quote_identifier(schema_name)?,
            state_table
        ))
        .execute(&mut *transaction)
        .await?;
        sqlx::query(&format!(
            "CREATE TRIGGER nudge_bump_access_group_generation \
             AFTER INSERT OR UPDATE OR DELETE ON {}.{} FOR EACH ROW \
             EXECUTE FUNCTION {}.nudge_bump_access_group_generation()",
            quote_identifier(schema_name)?,
            quote_identifier(&member.shape.table_name)?,
            quote_identifier(schema_name)?
        ))
        .execute(&mut *transaction)
        .await?;
    }
    if !trigger_exists(
        transaction,
        "core",
        "featureFlag",
        "nudge_invalidate_feature_flag_access_groups",
    )
    .await?
    {
        sqlx::query(&format!(
            "CREATE OR REPLACE FUNCTION core.nudge_invalidate_feature_flag_access_groups() RETURNS trigger \
             LANGUAGE plpgsql AS $$ DECLARE workspace_id uuid; workspace_schema text; flag_key text; BEGIN \
             workspace_id := CASE WHEN TG_OP = 'DELETE' THEN OLD.\"workspaceId\" ELSE NEW.\"workspaceId\" END; \
             flag_key := CASE WHEN TG_OP = 'DELETE' THEN OLD.key ELSE NEW.key END; \
             IF flag_key NOT IN ({}) THEN \
               IF TG_OP = 'UPDATE' AND OLD.key IN ({}) THEN flag_key := OLD.key; \
               ELSE RETURN NULL; END IF; \
             END IF; \
             SELECT \"databaseSchema\" INTO workspace_schema FROM core.workspace WHERE id = workspace_id; \
             IF workspace_schema IS NOT NULL AND to_regclass(format('%I.%I', workspace_schema, '{{}}')) IS NOT NULL THEN \
               EXECUTE format('UPDATE %I.\"{{}}\" SET generation = generation + 1 WHERE id = 1', workspace_schema); \
             END IF; RETURN NULL; END $$",
            ACCESS_GROUP_FEATURE_FLAGS
                .iter()
                .map(|flag| format!("'{flag}'"))
                .collect::<Vec<_>>()
                .join(", "),
            ACCESS_GROUP_FEATURE_FLAGS
                .iter()
                .map(|flag| format!("'{flag}'"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "CREATE TRIGGER nudge_invalidate_feature_flag_access_groups \
             AFTER INSERT OR UPDATE OR DELETE ON core.\"featureFlag\" FOR EACH ROW \
             EXECUTE FUNCTION core.nudge_invalidate_feature_flag_access_groups()",
        )
        .execute(&mut *transaction)
        .await?;
    }
    Ok(())
}

async fn trigger_exists(
    transaction: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    trigger_name: &str,
) -> Result<bool, AccessGroupError> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_trigger t \
         JOIN pg_class c ON c.oid = t.tgrelid JOIN pg_namespace n ON n.oid = c.relnamespace \
         WHERE n.nspname = $1 AND c.relname = $2 AND t.tgname = $3 AND NOT t.tgisinternal)",
    )
    .bind(schema_name)
    .bind(table_name)
    .bind(trigger_name)
    .fetch_one(&mut *transaction)
    .await?)
}

async fn bump_access_group_generation(
    transaction: &mut PgConnection,
    schema: &RunnerSchema,
) -> Result<(), AccessGroupError> {
    let company = schema
        .object(COMPANY_OBJECT_NAME)
        .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
    sqlx::query(&format!(
        "UPDATE {}.{} SET generation = generation + 1 WHERE id = 1",
        quote_identifier(&company.shape.schema_name)?,
        quote_identifier(ACCESS_GROUP_STATE_TABLE_NAME)?
    ))
    .execute(&mut *transaction)
    .await?;
    Ok(())
}

async fn invalidate_company_arrays(
    transaction: &mut PgConnection,
    schema: &RunnerSchema,
    record_ids: Option<&[Uuid]>,
) -> Result<(), AccessGroupError> {
    let company = schema
        .object(COMPANY_OBJECT_NAME)
        .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
    let schema_name = company.shape.schema_name.clone();
    let filter = if let Some(_ids) = record_ids.filter(|ids| !ids.is_empty()) {
        format!("WHERE {} = ANY($1)", quote_identifier("id")?)
    } else {
        String::new()
    };
    let sql = format!(
        "UPDATE {}.{} SET {} = NULL, {} = NULL {filter}",
        quote_identifier(&schema_name)?,
        quote_identifier(&company.shape.table_name)?,
        quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME)?,
        quote_identifier(ACCESS_GROUP_GENERATION_COLUMN_NAME)?
    );
    let mut query = sqlx::query(&sql);
    if record_ids.is_some_and(|ids| !ids.is_empty()) {
        query = query.bind(record_ids.unwrap_or_default());
    }
    query.execute(&mut *transaction).await?;
    Ok(())
}

fn build_runner_schema(
    workspace_id: &Uuid,
    maps: &FlatWorkspaceMaps,
) -> Result<RunnerSchema, AccessGroupError> {
    let objects = maps
        .object_metadata_maps
        .by_universal_identifier
        .values()
        .cloned()
        .collect::<Vec<_>>();
    RunnerSchema::build(
        workspace_id,
        &objects,
        &maps.field_metadata_maps,
        &maps.index_metadata_maps,
    )
    .map_err(|error| AccessGroupError::invalid(error.to_string()))
}

async fn load_workspace_members(
    connection: &mut PgConnection,
    schema: &RunnerSchema,
    workspace_id: Uuid,
) -> Result<Vec<(Uuid, Uuid)>, AccessGroupError> {
    let member_model = schema
        .object(WORKSPACE_MEMBER_OBJECT_NAME)
        .map_err(|error| AccessGroupError::unsupported(error.to_string()))?;
    let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(&format!(
        "SELECT DISTINCT rt.\"roleId\", member.id \
         FROM core.\"roleTarget\" rt \
         JOIN core.\"userWorkspace\" membership ON membership.id = rt.\"userWorkspaceId\" \
         JOIN {}.{} member ON member.\"userId\" = membership.\"userId\" \
         WHERE rt.\"workspaceId\" = $1 AND membership.\"workspaceId\" = $1 \
           AND membership.\"deletedAt\" IS NULL AND rt.\"isSystemSideEffect\" = false \
           AND member.\"deletedAt\" IS NULL",
        quote_identifier(&member_model.shape.schema_name)?,
        quote_identifier(&member_model.shape.table_name)?
    ))
    .bind(workspace_id)
    .fetch_all(&mut *connection)
    .await?;
    Ok(rows)
}

async fn load_member_field_values(
    connection: &mut PgConnection,
    schema: &RunnerSchema,
    member_id: Uuid,
    predicates: &[Value],
) -> Result<Map<String, Value>, AccessGroupError> {
    let mut values = Map::new();
    for predicate in predicates {
        let Some(member_field_id) = optional_uuid(predicate, "workspaceMemberFieldMetadataId")?
        else {
            continue;
        };
        if optional_uuid(predicate, "workspaceMemberRelationFieldMetadataId")?.is_some() {
            return Err(AccessGroupError::unsupported(
                "Workspace member relation values are not materialized",
            ));
        }
        let key = member_field_key(predicate)?;
        if values.contains_key(&key) {
            continue;
        }
        let field = schema.field_metadata(member_field_id).ok_or_else(|| {
            AccessGroupError::unsupported("Workspace member field metadata not found")
        })?;
        let member_model = schema
            .object_by_id(&field.object_metadata_id)
            .map_err(|error| AccessGroupError::unsupported(error.to_string()))?;
        if member_model.object.name_singular != WORKSPACE_MEMBER_OBJECT_NAME {
            return Err(AccessGroupError::unsupported(
                "Workspace member bound field does not belong to workspaceMember",
            ));
        }
        let sub_field = optional_string(predicate, "workspaceMemberSubFieldName");
        let column_name = sub_field
            .as_ref()
            .map(|sub_field| compute_composite_column_name(&field.name, sub_field))
            .unwrap_or_else(|| field.name.clone());
        let table = format!(
            "{}.{}",
            quote_identifier(&member_model.shape.schema_name)?,
            quote_identifier(&member_model.shape.table_name)?
        );
        let deleted_predicate = if member_model.shape.has_deleted_at_column {
            "AND member.\"deletedAt\" IS NULL"
        } else {
            ""
        };
        let value = sqlx::query_scalar::<_, Option<Value>>(&format!(
            "SELECT to_jsonb(member)->$2 FROM {table} member WHERE member.id = $1 {deleted_predicate}"
        ))
        .bind(member_id)
        .bind(column_name)
        .fetch_optional(&mut *connection)
        .await?
        .flatten()
        .ok_or_else(|| {
            AccessGroupError::invalid(format!(
                "Missing, deleted, or null workspace member value {key}"
            ))
        })?;
        values.insert(key, value);
    }
    Ok(values)
}

fn read_rules_supported(
    schema: &RunnerSchema,
    object_name: &str,
    object: &ObjectRecordsPermissions,
) -> bool {
    object
        .row_level_records_permissions
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|rule| rule.can_read_records)
        .all(|rule| crate::query::runner::rls::row_level_rule_supported(schema, object_name, rule))
}

fn member_predicates(object: &ObjectRecordsPermissions) -> Vec<Value> {
    object
        .row_level_records_permissions
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|rule| rule.can_read_records)
        .flat_map(|rule| rule.predicates.iter().cloned())
        .filter(|predicate| {
            optional_uuid(predicate, "workspaceMemberFieldMetadataId")
                .ok()
                .flatten()
                .is_some()
        })
        .collect()
}

pub(crate) fn build_role_access_groups(
    object_metadata_id: Uuid,
    object: &ObjectRecordsPermissions,
    operation: ObjectRecordsOperationType,
    member_values: Option<&Map<String, Value>>,
    relation_hop_enabled: bool,
) -> Result<Vec<AccessGroupWithoutCompactId>, AccessGroupError> {
    let mut groups = Vec::new();
    if object.operation_allowed(operation) {
        let shape = resolve_rule_shape(
            object_metadata_id,
            None,
            &object.unreadable_fields,
            relation_hop_enabled,
        );
        groups.push(AccessGroupWithoutCompactId {
            object_metadata_id,
            group_type: AccessGroupType::AllRecords,
            group_hash: compute_access_group_hash(&shape.rule_shape_hash, SplitValue::AllRecords),
            rule_shape_hash: shape.rule_shape_hash,
            unreadable_field_ids: shape.unreadable_field_ids,
            member_field_values: None,
            value: None,
        });
    }
    let rules = object
        .row_level_records_permissions
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|rule| rule_allows(rule, operation))
        .collect::<Vec<_>>();
    for rule in rules {
        let shape = resolve_rule_shape(
            object_metadata_id,
            Some(rule),
            &object.unreadable_fields,
            relation_hop_enabled,
        );
        if let Some(split_predicate) = shape.split_predicate.as_ref() {
            let values = split_predicate
                .get("value")
                .and_then(Value::as_array)
                .ok_or_else(|| AccessGroupError::invalid("Literal split value is not an array"))?;
            let mut seen = HashSet::new();
            for value in values {
                let value = value
                    .as_str()
                    .ok_or_else(|| {
                        AccessGroupError::invalid("Literal split value is not a string")
                    })?
                    .to_string();
                if !seen.insert(value.clone()) {
                    continue;
                }
                groups.push(AccessGroupWithoutCompactId {
                    object_metadata_id,
                    group_type: AccessGroupType::RlsLiteralValue,
                    group_hash: compute_access_group_hash(
                        &shape.rule_shape_hash,
                        SplitValue::Literal(value.clone()),
                    ),
                    rule_shape_hash: shape.rule_shape_hash.clone(),
                    unreadable_field_ids: shape.unreadable_field_ids.clone(),
                    member_field_values: None,
                    value: Some(value),
                });
            }
            continue;
        }
        let bound_predicates = rule
            .predicates
            .iter()
            .filter(|predicate| {
                optional_uuid(predicate, "workspaceMemberFieldMetadataId")
                    .ok()
                    .flatten()
                    .is_some()
            })
            .cloned()
            .collect::<Vec<_>>();
        if bound_predicates.is_empty() {
            groups.push(AccessGroupWithoutCompactId {
                object_metadata_id,
                group_type: AccessGroupType::RlsRule,
                group_hash: compute_access_group_hash(&shape.rule_shape_hash, SplitValue::RlsRule),
                rule_shape_hash: shape.rule_shape_hash,
                unreadable_field_ids: shape.unreadable_field_ids,
                member_field_values: None,
                value: None,
            });
            continue;
        }
        let Some(member_values) = member_values else {
            continue;
        };
        for values in build_member_field_value_combinations(&bound_predicates, member_values)? {
            let member_field_values = Value::Object(values.clone());
            groups.push(AccessGroupWithoutCompactId {
                object_metadata_id,
                group_type: AccessGroupType::RlsMemberValue,
                group_hash: compute_access_group_hash(
                    &shape.rule_shape_hash,
                    SplitValue::Member(values.clone()),
                ),
                rule_shape_hash: shape.rule_shape_hash.clone(),
                unreadable_field_ids: shape.unreadable_field_ids.clone(),
                member_field_values: Some(member_field_values),
                value: None,
            });
        }
    }
    Ok(groups)
}

fn build_member_field_value_combinations(
    predicates: &[Value],
    member_values: &Map<String, Value>,
) -> Result<Vec<Map<String, Value>>, AccessGroupError> {
    let mut keys = Vec::new();
    let mut seen_keys = HashSet::new();
    for predicate in predicates {
        let key = member_field_key(predicate)?;
        if seen_keys.insert(key.clone()) {
            keys.push(key);
        }
    }
    let mut combinations = vec![Map::new()];
    for key in keys {
        let value = member_values.get(&key).cloned().ok_or_else(|| {
            AccessGroupError::invalid(format!("Missing member field value {key}"))
        })?;
        let matching = predicates
            .iter()
            .filter(|predicate| member_field_key(predicate).ok().as_deref() == Some(key.as_str()))
            .collect::<Vec<_>>();
        if matching.iter().any(|predicate| {
            predicate
                .get("operand")
                .and_then(Value::as_str)
                .is_some_and(|operand| matches!(operand, "IS_EMPTY" | "IS_NOT_EMPTY"))
        }) {
            return Err(AccessGroupError::unsupported(
                "Member-bound emptiness predicates are not materialized",
            ));
        }
        if value.is_null() || value.as_array().is_some_and(Vec::is_empty) {
            return Err(AccessGroupError::invalid(format!(
                "Missing, null, or empty workspace member value {key}"
            )));
        }
        let can_split = matching.len() == 1
            && matching[0]
                .get("operand")
                .and_then(Value::as_str)
                .is_some_and(|operand| matches!(operand, "IS" | "CONTAINS"));
        let values = split_member_field_value(value, can_split);
        let mut next = Vec::new();
        for combination in &combinations {
            for value in &values {
                let mut combined = combination.clone();
                combined.insert(key.clone(), value.clone());
                next.push(combined);
            }
        }
        combinations = next;
    }
    Ok(combinations)
}

fn split_member_field_value(value: Value, can_split: bool) -> Vec<Value> {
    let Value::Array(values) = value else {
        return vec![value];
    };
    if !can_split {
        let mut values = values;
        values.sort_by(js_value_sort);
        return vec![Value::Array(values)];
    }
    if values.is_empty() {
        vec![Value::Null]
    } else {
        values
    }
}

fn resolve_rule_shape(
    object_metadata_id: Uuid,
    rule: Option<&RowLevelRecordsPermissions>,
    object_unreadable_fields: &[String],
    relation_hop_enabled: bool,
) -> RuleShape {
    let split_predicate = rule.and_then(find_split_predicate);
    let mut unreadable = object_unreadable_fields
        .iter()
        .filter_map(|field| Uuid::parse_str(field).ok())
        .collect::<Vec<_>>();
    if let Some(rule) = rule {
        unreadable.extend(
            rule.unreadable_fields
                .iter()
                .filter_map(|field| Uuid::parse_str(field).ok()),
        );
    }
    unreadable.sort();
    unreadable.dedup();
    let split_predicate_id = split_predicate
        .as_ref()
        .and_then(|predicate| predicate.get("id"))
        .and_then(Value::as_str);
    let rule_shape_hash = compute_rule_shape_hash(
        object_metadata_id,
        rule,
        split_predicate_id,
        &unreadable,
        relation_hop_enabled,
    );
    RuleShape {
        rule_shape_hash,
        unreadable_field_ids: unreadable,
        split_predicate,
    }
}

fn find_split_predicate(rule: &RowLevelRecordsPermissions) -> Option<Value> {
    if rule.predicates.iter().any(|predicate| {
        optional_uuid(predicate, "workspaceMemberFieldMetadataId")
            .ok()
            .flatten()
            .is_some()
    }) {
        return None;
    }
    let mut candidates = rule
        .predicates
        .iter()
        .filter(|predicate| {
            predicate.get("operand").and_then(Value::as_str) == Some("IS")
                && predicate
                    .get("value")
                    .and_then(Value::as_array)
                    .is_some_and(|values| {
                        values
                            .iter()
                            .all(|value| value.as_str().is_some_and(|value| !value.is_empty()))
                    })
                && predicate_at_root_conjunction(predicate, &rule.predicate_groups)
        })
        .cloned()
        .collect::<Vec<_>>();
    (candidates.len() == 1).then(|| candidates.remove(0))
}

fn predicate_at_root_conjunction(predicate: &Value, groups: &[Value]) -> bool {
    let mut group_id = optional_string(predicate, "rowLevelPermissionPredicateGroupId");
    while let Some(current) = group_id {
        let Some(group) = groups
            .iter()
            .find(|group| optional_string(group, "id").as_deref() == Some(current.as_str()))
        else {
            return false;
        };
        if group.get("logicalOperator").and_then(Value::as_str) != Some("AND") {
            return false;
        }
        group_id = optional_string(group, "parentRowLevelPermissionPredicateGroupId");
    }
    true
}

fn compute_rule_shape_hash(
    object_metadata_id: Uuid,
    rule: Option<&RowLevelRecordsPermissions>,
    split_predicate_id: Option<&str>,
    unreadable_field_ids: &[Uuid],
    relation_hop_enabled: bool,
) -> String {
    let serialized_rule = rule.map(|rule| {
        let predicates = rule
            .predicates
            .iter()
            .map(|predicate| {
                if split_predicate_id
                    .as_ref()
                    .is_some_and(|id| optional_string(predicate, "id").as_deref() == Some(*id))
                {
                    let mut predicate = predicate.clone();
                    predicate
                        .as_object_mut()
                        .expect("predicate object")
                        .insert(String::from("value"), Value::Null);
                    predicate
                } else {
                    predicate.clone()
                }
            })
            .collect::<Vec<_>>();
        serialize_predicate_group(
            None,
            &predicates,
            &rule.predicate_groups,
            relation_hop_enabled,
        )
    });
    let mut unreadable = unreadable_field_ids
        .iter()
        .map(Uuid::to_string)
        .collect::<Vec<_>>();
    unreadable.sort_by(|first, second| js_string_sort(first, second));
    hash_json(json!([
        object_metadata_id.to_string(),
        serialized_rule,
        unreadable
    ]))
}

fn serialize_predicate_group(
    parent_group_id: Option<&str>,
    predicates: &[Value],
    groups: &[Value],
    relation_hop_enabled: bool,
) -> Value {
    let mut serialized_predicates = predicates
        .iter()
        .filter(|predicate| {
            optional_string(predicate, "rowLevelPermissionPredicateGroupId").as_deref()
                == parent_group_id
        })
        .map(|predicate| {
            let mut entries = Vec::new();
            if let Some(relation_field_id) = optional_uuid(predicate, "relationFieldMetadataId")
                .expect("valid relation field id")
            {
                entries.push(Value::String(relation_field_id.to_string()));
                entries.push(Value::Bool(relation_hop_enabled));
            }
            entries.push(Value::String(
                required_uuid(predicate, "fieldMetadataId")
                    .expect("valid field id")
                    .to_string(),
            ));
            entries.push(
                optional_string(predicate, "subFieldName")
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            entries.push(
                required_string(predicate, "operand")
                    .expect("valid operand")
                    .into(),
            );
            entries.push(predicate.get("value").cloned().unwrap_or(Value::Null));
            entries.push(
                optional_uuid(predicate, "workspaceMemberFieldMetadataId")
                    .expect("valid member field id")
                    .map(|id| Value::String(id.to_string()))
                    .unwrap_or(Value::Null),
            );
            entries.push(
                optional_string(predicate, "workspaceMemberSubFieldName")
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
            if let Some(relation_field_id) =
                optional_uuid(predicate, "workspaceMemberRelationFieldMetadataId")
                    .expect("valid member relation field id")
            {
                entries.push(Value::String(relation_field_id.to_string()));
            }
            serde_json::to_string(&Value::Array(entries)).expect("predicate serializes")
        })
        .collect::<Vec<_>>();
    serialized_predicates.sort_by(|first, second| js_string_sort(first, second));
    let mut serialized_groups = groups
        .iter()
        .filter(|group| {
            optional_string(group, "parentRowLevelPermissionPredicateGroupId").as_deref()
                == parent_group_id
        })
        .map(|group| {
            let group_id = required_string(group, "id").expect("valid group id");
            serde_json::to_string(&json!([
                required_string(group, "logicalOperator").expect("valid logical operator"),
                serialize_predicate_group(Some(group_id), predicates, groups, relation_hop_enabled,)
            ]))
            .expect("predicate group serializes")
        })
        .collect::<Vec<_>>();
    serialized_groups.sort_by(|first, second| js_string_sort(first, second));
    json!({
        "predicates": serialized_predicates,
        "predicateGroups": serialized_groups,
    })
}

fn compute_access_group_hash(rule_shape_hash: &str, split_value: SplitValue) -> String {
    let (group_type, value) = match split_value {
        SplitValue::AllRecords => ("ALL_RECORDS", Value::Null),
        SplitValue::RlsRule => ("RLS_RULE", Value::Null),
        SplitValue::Literal(value) => ("RLS_LITERAL_VALUE", Value::String(value)),
        SplitValue::Member(values) => {
            let mut entries = values
                .into_iter()
                .map(|(key, value)| Value::Array(vec![Value::String(key), value]))
                .collect::<Vec<_>>();
            entries.sort_by(|first, second| {
                js_locale_compare(
                    first[0].as_str().expect("member key"),
                    second[0].as_str().expect("member key"),
                )
            });
            ("RLS_MEMBER_VALUE", Value::Array(entries))
        }
    };
    hash_json(json!([group_type, rule_shape_hash, value]))
}

fn hash_json(value: Value) -> String {
    let serialized = serde_json::to_string(&canonicalize_js_value(value))
        .expect("access group hash input serializes");
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn canonicalize_js_value(value: Value) -> Value {
    match value {
        Value::Number(number) => {
            if let Some(value) = number.as_f64() {
                if value == 0.0 {
                    return Value::from(0);
                }
                if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
                    return Value::from(value as i64);
                }
            }
            Value::Number(number)
        }
        Value::Array(values) => {
            Value::Array(values.into_iter().map(canonicalize_js_value).collect())
        }
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, canonicalize_js_value(value)))
                .collect(),
        ),
        _ => value,
    }
}

fn js_string_sort(first: &str, second: &str) -> std::cmp::Ordering {
    first.encode_utf16().cmp(second.encode_utf16())
}

fn js_locale_compare(first: &str, second: &str) -> std::cmp::Ordering {
    thread_local! {
        static COLLATOR: Collator = Collator::try_new(
            CollatorPreferences::from(icu_locale_core::locale!("en-US")),
            CollatorOptions::default(),
        )
        .expect("default ICU collator")
        .static_to_owned()
    }
    COLLATOR.with(|collator| {
        collator.as_borrowed().compare_utf16(
            &first.encode_utf16().collect::<Vec<_>>(),
            &second.encode_utf16().collect::<Vec<_>>(),
        )
    })
}

fn js_value_sort(first: &Value, second: &Value) -> std::cmp::Ordering {
    js_string_sort(
        &serde_json::to_string(first).expect("JSON value serializes"),
        &serde_json::to_string(second).expect("JSON value serializes"),
    )
}

fn group_rule_for_group(
    object: &ObjectRecordsPermissions,
    group: &AccessGroupWithoutCompactId,
) -> Result<Option<RowLevelRecordsPermissions>, AccessGroupError> {
    if group.group_type == AccessGroupType::AllRecords {
        return Ok(None);
    }
    let rule = object
        .row_level_records_permissions
        .as_ref()
        .into_iter()
        .flatten()
        .find(|rule| {
            resolve_rule_shape(
                group.object_metadata_id,
                Some(rule),
                &object.unreadable_fields,
                false,
            )
            .rule_shape_hash
                == group.rule_shape_hash
        })
        .cloned()
        .ok_or_else(|| AccessGroupError::invalid("Access group rule shape not found"))?;
    Ok(Some(rule))
}

fn rule_for_stored_group(
    group: &AccessGroup,
    rule_shapes: &HashMap<String, RowLevelRecordsPermissions>,
) -> Result<Option<RowLevelRecordsPermissions>, AccessGroupError> {
    if group.group.group_type == AccessGroupType::AllRecords {
        return Ok(None);
    }
    rule_shapes
        .get(&group.group.rule_shape_hash)
        .cloned()
        .map(Some)
        .ok_or_else(|| AccessGroupError::invalid("Stored access group rule shape not found"))
}

fn stored_group_rule(
    group: &AccessGroup,
    rule: &RowLevelRecordsPermissions,
) -> Result<RowLevelRecordsPermissions, AccessGroupError> {
    let mut rule = rule.clone();
    match group.group.group_type {
        AccessGroupType::AllRecords => {}
        AccessGroupType::RlsRule => {}
        AccessGroupType::RlsLiteralValue => {
            let split = find_split_predicate(&rule).ok_or_else(|| {
                AccessGroupError::invalid("Literal group split predicate not found")
            })?;
            let split_id = required_string(&split, "id")?.to_string();
            let value = group
                .group
                .value
                .clone()
                .ok_or_else(|| AccessGroupError::invalid("Literal group value is missing"))?;
            for predicate in &mut rule.predicates {
                if optional_string(predicate, "id").as_deref() == Some(split_id.as_str()) {
                    predicate
                        .as_object_mut()
                        .expect("predicate object")
                        .insert(String::from("value"), json!([value]));
                }
            }
        }
        AccessGroupType::RlsMemberValue => {
            let values = group
                .group
                .member_field_values
                .as_ref()
                .and_then(Value::as_object)
                .ok_or_else(|| AccessGroupError::invalid("Member group values are missing"))?;
            for predicate in &mut rule.predicates {
                let Some(member_field_id) =
                    optional_uuid(predicate, "workspaceMemberFieldMetadataId")?
                else {
                    continue;
                };
                let key = member_field_key(predicate)?;
                let value = values
                    .get(&key)
                    .cloned()
                    .ok_or_else(|| AccessGroupError::invalid("Member group value is missing"))?;
                let predicate = predicate.as_object_mut().expect("predicate object");
                predicate.insert(String::from("value"), value);
                predicate.insert(String::from("workspaceMemberFieldMetadataId"), Value::Null);
                predicate.insert(String::from("workspaceMemberSubFieldName"), Value::Null);
                predicate.insert(
                    String::from("workspaceMemberRelationFieldMetadataId"),
                    Value::Null,
                );
                let _ = member_field_id;
            }
        }
    }
    Ok(rule)
}

async fn insert_access_group(
    transaction: &mut PgConnection,
    schema_name: &str,
    group: &AccessGroupWithoutCompactId,
) -> Result<(), AccessGroupError> {
    sqlx::query(&format!(
        "INSERT INTO {}.{} ({}, {}, {}, {}, {}, {}, {}) \
         VALUES ($1, $2::text::{}.{}, $3, $4, $5::jsonb, $6, $7) \
         ON CONFLICT DO NOTHING",
        quote_identifier(schema_name)?,
        quote_identifier("accessGroup")?,
        quote_identifier("objectMetadataId")?,
        quote_identifier("type")?,
        quote_identifier("ruleShapeHash")?,
        quote_identifier("unreadableFieldIds")?,
        quote_identifier("memberFieldValues")?,
        quote_identifier("value")?,
        quote_identifier("groupHash")?,
        quote_identifier(schema_name)?,
        quote_identifier("accessGroup_type_enum")?
    ))
    .bind(group.object_metadata_id)
    .bind(group.group_type.as_str())
    .bind(&group.rule_shape_hash)
    .bind(&group.unreadable_field_ids)
    .bind(
        group
            .member_field_values
            .as_ref()
            .map(|value| value.to_string()),
    )
    .bind(&group.value)
    .bind(&group.group_hash)
    .execute(&mut *transaction)
    .await?;
    Ok(())
}

async fn load_company_access_groups(
    transaction: &mut PgConnection,
    schema_name: &str,
    object_metadata_id: Uuid,
) -> Result<Vec<AccessGroup>, AccessGroupError> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        i32,
        String,
        String,
        Vec<Uuid>,
        Option<Value>,
        Option<String>,
    )> = sqlx::query_as(&format!(
        "SELECT {}, {}, {}, {}, {}, {} FROM {}.{} WHERE {} = $1 ORDER BY {}",
        quote_identifier("compactId")?,
        quote_identifier("groupHash")?,
        quote_identifier("ruleShapeHash")?,
        quote_identifier("unreadableFieldIds")?,
        quote_identifier("memberFieldValues")?,
        quote_identifier("value")?,
        quote_identifier(schema_name)?,
        quote_identifier("accessGroup")?,
        quote_identifier("objectMetadataId")?,
        quote_identifier("compactId")?
    ))
    .bind(object_metadata_id)
    .fetch_all(&mut *transaction)
    .await?;
    let mut type_rows: Vec<(i32, String)> = sqlx::query_as(&format!(
        "SELECT {}, {}::text FROM {}.{} WHERE {} = $1 ORDER BY {}",
        quote_identifier("compactId")?,
        quote_identifier("type")?,
        quote_identifier(schema_name)?,
        quote_identifier("accessGroup")?,
        quote_identifier("objectMetadataId")?,
        quote_identifier("compactId")?
    ))
    .bind(object_metadata_id)
    .fetch_all(&mut *transaction)
    .await?;
    let mut groups = Vec::new();
    for (
        (compact_id, group_hash, rule_shape_hash, unreadable_field_ids, member_field_values, value),
        (_, group_type),
    ) in rows.into_iter().zip(type_rows.drain(..))
    {
        groups.push(AccessGroup {
            compact_id,
            group: AccessGroupWithoutCompactId {
                object_metadata_id,
                group_type: AccessGroupType::parse(&group_type)?,
                rule_shape_hash,
                unreadable_field_ids,
                member_field_values,
                value,
                group_hash,
            },
        });
    }
    Ok(groups)
}

#[allow(clippy::too_many_arguments)]
async fn recompute_company_arrays(
    transaction: &mut PgConnection,
    pool: &PgPool,
    schema: &RunnerSchema,
    schema_name: &str,
    company: &FlatObjectMetadata,
    groups: &[AccessGroup],
    rule_shapes: &HashMap<String, RowLevelRecordsPermissions>,
    record_ids: Option<&[Uuid]>,
) -> Result<usize, AccessGroupError> {
    if record_ids.is_some_and(|ids| ids.is_empty()) {
        return Ok(0);
    }
    let generation: i64 = sqlx::query_scalar(&format!(
        "SELECT generation FROM {}.{} WHERE id = 1",
        quote_identifier(schema_name)?,
        quote_identifier(ACCESS_GROUP_STATE_TABLE_NAME)?
    ))
    .fetch_one(&mut *transaction)
    .await?;
    let table_name = company_target_table(company);
    let table = format!(
        "{}.{}",
        quote_identifier(schema_name)?,
        quote_identifier(&table_name)?
    );
    let mut suffixes = Suffixes::new();
    let mut parameters = Map::new();
    let mut cases = Vec::new();
    for group in groups {
        if group.group.group_type == AccessGroupType::AllRecords {
            cases.push(group.compact_id.to_string());
            continue;
        }
        let rule = rule_for_stored_group(group, rule_shapes)?;
        let Some(rule) = rule else {
            cases.push(group.compact_id.to_string());
            continue;
        };
        let rule = stored_group_rule(group, &rule)?;
        let filter = crate::query::runner::rls::compile_access_group_rule(
            pool,
            schema,
            COMPANY_OBJECT_NAME,
            &rule,
            "record",
            &mut suffixes,
        )
        .await
        .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
        let condition = crate::query::runner::base::resolve_where_clauses(
            schema,
            COMPANY_OBJECT_NAME,
            "record",
            &filter.clauses,
        )
        .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
        parameters.extend(filter.params);
        cases.push(format!(
            "CASE WHEN ({condition}) THEN {}::integer END",
            group.compact_id
        ));
    }
    let expression = if cases.is_empty() {
        String::from("CAST('{}' AS integer[])")
    } else {
        format!(
            "ARRAY_REMOVE(CAST(ARRAY[{}] AS integer[]), NULL)",
            cases.join(", ")
        )
    };
    let record_filter = if let Some(ids) = record_ids {
        parameters.insert(
            String::from("recordIds"),
            Value::Array(ids.iter().map(|id| Value::String(id.to_string())).collect()),
        );
        "WHERE record.id = ANY(CAST(:recordIds AS uuid[]))"
    } else {
        ""
    };
    let sql = format!(
        "UPDATE {table} AS record SET {} = recomputed.\"accessGroupIds\", \
         {} = {generation} \
         FROM (SELECT record.id, {expression} AS \"accessGroupIds\" FROM {table} AS record {record_filter}) AS recomputed \
         WHERE record.id = recomputed.id AND \
           (record.{} IS DISTINCT FROM recomputed.\"accessGroupIds\" OR \
            record.{} IS DISTINCT FROM {generation}) \
         RETURNING record.id",
        quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME)?,
        quote_identifier(ACCESS_GROUP_GENERATION_COLUMN_NAME)?,
        quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME)?,
        quote_identifier(ACCESS_GROUP_GENERATION_COLUMN_NAME)?
    );
    let compiled = crate::query::compile::compile_named_parameters(&sql, &parameters)
        .map_err(|error| AccessGroupError::invalid(error.message()))?;
    let mut query = sqlx::query(&compiled.text);
    for value in &compiled.values {
        query = crate::query::runner::base::bind_json_value(query, value)
            .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
    }
    sqlx::query("SELECT set_config($1, 'on', true)")
        .bind(ACCESS_GROUP_REBUILD_SETTING)
        .execute(&mut *transaction)
        .await?;
    let rows = query.fetch_all(&mut *transaction).await?;
    if record_ids.is_none() {
        sqlx::query(&format!(
            "UPDATE {}.{} SET {} = $1::integer WHERE id = 1",
            quote_identifier(schema_name)?,
            quote_identifier(ACCESS_GROUP_STATE_TABLE_NAME)?,
            quote_identifier(ACCESS_GROUP_MATERIALIZED_GENERATION_COLUMN_NAME)?
        ))
        .bind(generation)
        .execute(&mut *transaction)
        .await?;
    }
    sqlx::query("SELECT set_config($1, 'off', true)")
        .bind(ACCESS_GROUP_REBUILD_SETTING)
        .execute(&mut *transaction)
        .await?;
    Ok(rows.len())
}

async fn find_materialized_compact_ids(
    pool: &PgPool,
    schema: &RunnerSchema,
    workspace_id: Uuid,
    object_metadata_id: Uuid,
    expected_groups: &[AccessGroupWithoutCompactId],
) -> Result<Option<Vec<i32>>, AccessGroupError> {
    if expected_groups.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let company = schema
        .object(COMPANY_OBJECT_NAME)
        .map_err(|error| AccessGroupError::invalid(error.to_string()))?;
    let schema_name = company.shape.schema_name.clone();
    let state_table = format!(
        "{}.{}",
        quote_identifier(&schema_name)?,
        quote_identifier(ACCESS_GROUP_STATE_TABLE_NAME)?
    );
    let generation_matches = format!(
        "(SELECT generation = {} FROM {} WHERE id = 1)",
        quote_identifier(ACCESS_GROUP_MATERIALIZED_GENERATION_COLUMN_NAME)?,
        state_table
    );
    let mut transaction = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtext($1))")
        .bind(format!("nudge-access-groups:{workspace_id}"))
        .execute(&mut *transaction)
        .await?;
    let storage_ready: bool = sqlx::query_scalar(
        "SELECT to_regclass($1) IS NOT NULL AND EXISTS( \
         SELECT 1 FROM information_schema.columns WHERE table_schema = $2 \
         AND table_name = $3 AND column_name = $4) AND EXISTS( \
         SELECT 1 FROM information_schema.columns WHERE table_schema = $2 \
         AND table_name = $5 AND column_name = $6)",
    )
    .bind(&state_table)
    .bind(&schema_name)
    .bind(&company.shape.table_name)
    .bind(ACCESS_GROUP_GENERATION_COLUMN_NAME)
    .bind(ACCESS_GROUP_STATE_TABLE_NAME)
    .bind(ACCESS_GROUP_MATERIALIZED_GENERATION_COLUMN_NAME)
    .fetch_one(&mut *transaction)
    .await?;
    if !storage_ready {
        return Ok(None);
    }
    let generation_materialized: bool =
        sqlx::query_scalar(&format!("SELECT COALESCE({generation_matches}, false)"))
            .fetch_one(&mut *transaction)
            .await?;
    if !generation_materialized {
        return Ok(None);
    }
    let stored =
        load_company_access_groups(&mut transaction, &schema_name, object_metadata_id).await?;
    let mut by_hash = stored
        .into_iter()
        .map(|group| (group.group.group_hash.clone(), group))
        .collect::<HashMap<_, _>>();
    let mut compact_ids = Vec::new();
    for expected in expected_groups {
        let Some(stored) = by_hash.remove(&expected.group_hash) else {
            return Ok(None);
        };
        if stored.group.group_type != expected.group_type
            || stored.group.rule_shape_hash != expected.rule_shape_hash
            || stored.group.unreadable_field_ids != expected.unreadable_field_ids
            || stored.group.member_field_values != expected.member_field_values
            || stored.group.value != expected.value
        {
            return Ok(None);
        }
        compact_ids.push(stored.compact_id);
    }
    let arrays_complete: bool = sqlx::query_scalar(&company_dirty_probe_sql(
        &schema_name,
        &company.shape.table_name,
        &generation_matches,
    )?)
    .fetch_one(&mut *transaction)
    .await?;
    if !arrays_complete {
        return Ok(None);
    }
    compact_ids.sort();
    compact_ids.dedup();
    Ok(Some(compact_ids))
}

fn company_dirty_probe_sql(
    schema_name: &str,
    table_name: &str,
    generation_matches: &str,
) -> Result<String, AccessGroupError> {
    Ok(format!(
        "SELECT COALESCE({generation_matches} AND NOT EXISTS(SELECT 1 FROM {}.{} \
         WHERE {} IS NULL OR {} IS NULL), false)",
        quote_identifier(schema_name)?,
        quote_identifier(table_name)?,
        quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME)?,
        quote_identifier(ACCESS_GROUP_GENERATION_COLUMN_NAME)?
    ))
}

fn company_target_table(company: &FlatObjectMetadata) -> String {
    crate::metadata::names::compute_object_target_table(
        &company.name_singular,
        &company.application_universal_identifier,
    )
}

fn rule_allows(rule: &RowLevelRecordsPermissions, operation: ObjectRecordsOperationType) -> bool {
    match operation {
        ObjectRecordsOperationType::Create => rule.can_create_records,
        ObjectRecordsOperationType::Read => rule.can_read_records,
        ObjectRecordsOperationType::Update => rule.can_update_records,
        ObjectRecordsOperationType::SoftDelete => rule.can_soft_delete_records,
        ObjectRecordsOperationType::Destroy => rule.can_destroy_records,
    }
}

fn member_field_key(predicate: &Value) -> Result<String, AccessGroupError> {
    let member_field_id = optional_uuid(predicate, "workspaceMemberFieldMetadataId")?
        .ok_or_else(|| AccessGroupError::invalid("Workspace member field id is missing"))?;
    let sub_field = optional_string(predicate, "workspaceMemberSubFieldName").unwrap_or_default();
    let key = format!("{member_field_id}:{sub_field}");
    if let Some(relation_field_id) =
        optional_uuid(predicate, "workspaceMemberRelationFieldMetadataId")?
    {
        Ok(format!("{relation_field_id}>{key}"))
    } else {
        Ok(key)
    }
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, AccessGroupError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AccessGroupError::invalid(format!("Missing {key}")))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn required_uuid(value: &Value, key: &str) -> Result<Uuid, AccessGroupError> {
    Uuid::parse_str(required_string(value, key)?)
        .map_err(|_| AccessGroupError::invalid(format!("Invalid {key}")))
}

fn optional_uuid(value: &Value, key: &str) -> Result<Option<Uuid>, AccessGroupError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|value| {
            Uuid::parse_str(value).map_err(|_| AccessGroupError::invalid(format!("Invalid {key}")))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::model::ObjectRecordsPermissions;
    use crate::permissions::model::RowLevelRecordsPermissions;

    const OBJECT_ID: &str = "20202020-0000-4000-8000-000000000001";
    const FIELD_ID: &str = "20202020-0000-4000-8000-000000000002";
    const MEMBER_FIELD_ID: &str = "20202020-0000-4000-8000-000000000003";

    fn predicate(value: Value) -> Value {
        json!({
            "id": "20202020-0000-4000-8000-000000000004",
            "fieldMetadataId": FIELD_ID,
            "objectMetadataId": OBJECT_ID,
            "operand": "IS",
            "value": value,
            "workspaceMemberFieldMetadataId": null,
            "workspaceMemberSubFieldName": null,
            "workspaceMemberRelationFieldMetadataId": null,
            "rowLevelPermissionPredicateGroupId": null,
        })
    }

    fn rule(predicates: Vec<Value>) -> RowLevelRecordsPermissions {
        RowLevelRecordsPermissions {
            rule_id: String::from("20202020-0000-4000-8000-000000000005"),
            can_read_records: true,
            predicates,
            ..Default::default()
        }
    }

    fn object(rule: RowLevelRecordsPermissions) -> ObjectRecordsPermissions {
        ObjectRecordsPermissions {
            object_metadata_id: OBJECT_ID.to_string(),
            row_level_records_permissions: Some(vec![rule]),
            can_read_some_records: true,
            ..Default::default()
        }
    }

    #[test]
    fn hashes_match_the_pinned_twenty_vectors() {
        let object_id = Uuid::parse_str(OBJECT_ID).unwrap();
        let rule = rule(vec![predicate(json!("ENGINEERING"))]);
        let shape = resolve_rule_shape(object_id, Some(&rule), &[], false);
        assert_eq!(
            shape.rule_shape_hash,
            "bd422171c406fadc49b02dd7207a91b05ede5c1e00f3058d0f884776cceede8d"
        );
        assert_eq!(
            compute_access_group_hash(&shape.rule_shape_hash, SplitValue::AllRecords),
            "6e0bb34345b09396e69ab5b94c8a390ce1cf0964a36ab74b30d79c4ef4f185df"
        );
        assert_eq!(
            compute_access_group_hash(&shape.rule_shape_hash, SplitValue::RlsRule),
            "8575852cb9c057c03f0e96697c972cd064d34b52c0407b124f455883e059b608"
        );
        assert_eq!(
            compute_access_group_hash(
                &shape.rule_shape_hash,
                SplitValue::Literal(String::from("ENGINEERING"))
            ),
            "97d6757a3ce496dba74633b0a82cd7ae6b0f8ae624bfe231bad3809859ca8d8a"
        );
        assert_eq!(
            compute_access_group_hash(
                &shape.rule_shape_hash,
                SplitValue::Member(Map::from_iter([(
                    format!("{MEMBER_FIELD_ID}:"),
                    json!("sales")
                )]))
            ),
            "f9c0c304ad4bb96ee6003c9582b7bdb340855578e2f64cbe253576c6c5d9fc29"
        );
    }

    #[test]
    fn member_hash_vectors_match_node_locale_compare_for_case_accents_and_nested_values() {
        let rule_shape_hash = "bd422171c406fadc49b02dd7207a91b05ede5c1e00f3058d0f884776cceede8d";
        let values = Map::from_iter([
            (String::from("z"), json!("lower")),
            (
                String::from("A"),
                json!({"nested": ["é", "e\u{301}", "Z", "a"], "amount": 1.0}),
            ),
            (String::from("á"), json!(["sales", "support"])),
            (String::from("a"), json!(false)),
            (String::from("Å"), json!({"z": 1, "a": 2})),
            (String::from("ä"), json!(2)),
            (String::from("b"), json!(true)),
        ]);
        assert_eq!(
            compute_access_group_hash(rule_shape_hash, SplitValue::Member(values.clone())),
            "955a9996d10bcc9e709936461b792cff70b12f5d80721d2d4fe4ae210ebd87d3"
        );
        let reversed = values.into_iter().rev().collect();
        assert_eq!(
            compute_access_group_hash(rule_shape_hash, SplitValue::Member(reversed)),
            "955a9996d10bcc9e709936461b792cff70b12f5d80721d2d4fe4ae210ebd87d3"
        );

        let multi_key = Map::from_iter([
            (
                String::from("20202020-0000-4000-8000-000000000003:"),
                json!("sales"),
            ),
            (
                String::from("20202020-0000-4000-8000-000000000006:firstName"),
                json!("Élodie"),
            ),
            (
                String::from("20202020-0000-4000-8000-000000000007:"),
                json!(["ä", "a", "Å"]),
            ),
            (
                String::from(
                    "20202020-0000-4000-8000-000000000008>20202020-0000-4000-8000-000000000009:",
                ),
                json!({"currencyCode": "EUR", "amountMicros": 1200}),
            ),
        ]);
        assert_eq!(
            compute_access_group_hash(rule_shape_hash, SplitValue::Member(multi_key)),
            "09d1a63c44ebc138b7feebd6827d744fc68723b1006296122e5bbec7e5255065"
        );

        let mut accented = [
            "z", "A", "á", "a", "Å", "ä", "b", "é", "e\u{301}", "😀", "\u{e000}",
        ]
        .map(String::from);
        accented.sort_by(|first, second| js_locale_compare(first, second));
        assert_eq!(
            accented,
            ["😀", "a", "A", "á", "Å", "ä", "b", "é", "e\u{301}", "z", "\u{e000}"]
                .map(String::from)
        );
        assert_eq!(
            js_locale_compare("é", "e\u{301}"),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn empty_literal_lists_follow_twenty_every_and_create_no_rule_group() {
        let object_id = Uuid::parse_str(OBJECT_ID).unwrap();
        let empty_rule = rule(vec![predicate(json!([]))]);
        assert!(find_split_predicate(&empty_rule).is_some());
        let groups = build_role_access_groups(
            object_id,
            &object(empty_rule),
            ObjectRecordsOperationType::Read,
            None,
            false,
        )
        .unwrap();
        assert!(groups.is_empty());
    }

    #[test]
    fn null_missing_and_empty_member_values_fail_closed_for_empty_operands() {
        let key = format!("{MEMBER_FIELD_ID}:");
        for operand in ["IS_EMPTY", "IS_NOT_EMPTY"] {
            let mut member_predicate = predicate(Value::Null);
            member_predicate.as_object_mut().unwrap().insert(
                "workspaceMemberFieldMetadataId".to_string(),
                json!(MEMBER_FIELD_ID),
            );
            member_predicate
                .as_object_mut()
                .unwrap()
                .insert("operand".to_string(), json!(operand));
            for values in [
                Map::from_iter([(key.clone(), Value::Null)]),
                Map::from_iter([(key.clone(), json!([]))]),
                Map::new(),
            ] {
                assert!(build_member_field_value_combinations(
                    std::slice::from_ref(&member_predicate),
                    &values,
                )
                .is_err());
            }
        }
    }

    #[test]
    fn literal_groups_are_deduplicated_and_exclude_the_split_value_from_shape() {
        let object_id = Uuid::parse_str(OBJECT_ID).unwrap();
        let permissions = object(rule(vec![predicate(json!(["sales", "sales", "support"]))]));
        let groups = build_role_access_groups(
            object_id,
            &permissions,
            ObjectRecordsOperationType::Read,
            None,
            false,
        )
        .unwrap();
        assert_eq!(groups.len(), 2);
        assert!(groups
            .iter()
            .all(|group| group.group_type == AccessGroupType::RlsLiteralValue));
        assert_eq!(
            groups
                .iter()
                .filter_map(|group| group.value.as_deref())
                .collect::<Vec<_>>(),
            vec!["sales", "support"]
        );
        let single = object(rule(vec![predicate(json!(["sales"]))]));
        let single_groups = build_role_access_groups(
            object_id,
            &single,
            ObjectRecordsOperationType::Read,
            None,
            false,
        )
        .unwrap();
        assert_eq!(groups[0].rule_shape_hash, single_groups[0].rule_shape_hash);
    }

    #[test]
    fn member_lists_split_only_for_single_is_or_contains_predicates() {
        let member_field_id = Uuid::parse_str(MEMBER_FIELD_ID).unwrap();
        let key = format!("{member_field_id}:");
        let mut member_predicate = predicate(Value::Null);
        member_predicate.as_object_mut().unwrap().insert(
            "workspaceMemberFieldMetadataId".to_string(),
            json!(MEMBER_FIELD_ID),
        );
        let values = Map::from_iter([(key.clone(), json!(["sales", "support"]))]);
        let combinations =
            build_member_field_value_combinations(&[member_predicate.clone()], &values).unwrap();
        assert_eq!(
            combinations,
            vec![
                Map::from_iter([(key.clone(), json!("sales"))]),
                Map::from_iter([(key.clone(), json!("support"))])
            ]
        );
        member_predicate
            .as_object_mut()
            .unwrap()
            .insert("operand".to_string(), json!("IS_NOT"));
        let combinations =
            build_member_field_value_combinations(&[member_predicate], &values).unwrap();
        assert_eq!(
            combinations,
            vec![Map::from_iter([(key, json!(["sales", "support"]))])]
        );
    }

    #[test]
    fn all_records_stays_a_separate_boolean_branch() {
        let object_id = Uuid::parse_str(OBJECT_ID).unwrap();
        let permissions = ObjectRecordsPermissions {
            object_metadata_id: OBJECT_ID.to_string(),
            can_read_all_records: true,
            row_level_records_permissions: Some(vec![rule(vec![predicate(json!("sales"))])]),
            can_read_some_records: true,
            ..Default::default()
        };
        let groups = build_role_access_groups(
            object_id,
            &permissions,
            ObjectRecordsOperationType::Read,
            None,
            false,
        )
        .unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].group_type, AccessGroupType::AllRecords);
        assert_eq!(groups[1].group_type, AccessGroupType::RlsRule);
        assert_ne!(groups[0].group_hash, groups[1].group_hash);
    }

    #[test]
    fn unreadable_fields_and_rule_shape_change_the_hash() {
        let object_id = Uuid::parse_str(OBJECT_ID).unwrap();
        let rule = rule(vec![predicate(json!("sales"))]);
        let first = resolve_rule_shape(object_id, Some(&rule), &[], false);
        let second = resolve_rule_shape(object_id, Some(&rule), &[FIELD_ID.to_string()], false);
        assert_ne!(first.rule_shape_hash, second.rule_shape_hash);
        assert_eq!(
            second.unreadable_field_ids,
            vec![Uuid::parse_str(FIELD_ID).unwrap()]
        );
        let mut moved = rule.clone();
        moved.predicates[0].as_object_mut().unwrap().insert(
            "rowLevelPermissionPredicateGroupId".to_string(),
            json!("group"),
        );
        moved.predicate_groups.push(json!({
            "id": "group",
            "parentRowLevelPermissionPredicateGroupId": null,
            "logicalOperator": "AND"
        }));
        assert_ne!(
            first.rule_shape_hash,
            resolve_rule_shape(object_id, Some(&moved), &[], false).rule_shape_hash
        );
    }

    #[test]
    fn private_and_sharing_flags_deny_non_system_company_access() {
        let mut strategy = CompanyAccessStrategy {
            workspace_id: Uuid::new_v4(),
            company_object_id: Uuid::new_v4(),
            principal_kind: PermissionPrincipalKind::WorkspaceMember,
            workspace_member_id: Some(Uuid::new_v4()),
            role_permissions: Vec::new(),
            enabled: true,
            relation_hop_enabled: false,
            overlays_enabled: true,
            limiting_roles: false,
        };
        assert!(strategy.denies_overlays());
        strategy.principal_kind = PermissionPrincipalKind::System;
        assert!(!strategy.denies_overlays());
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::auth::jwt_service::TokenType;
    use crate::auth::middleware::AuthContext;
    use crate::query::runner::test_support::DB_TEST_LOCK;
    use crate::query::runner::FilteredMutationKind;
    use crate::query::runner::FindManyArgs;

    struct CompanyFixture {
        seeded: crate::query::runner::test_support::SeededWorkspace,
        maps: FlatWorkspaceMaps,
        role_id: Uuid,
        application_id: Uuid,
        company_id: Uuid,
        name_field_id: Uuid,
        category_field_id: Uuid,
        member_id: Uuid,
        user_id: Uuid,
        user_workspace_id: Uuid,
    }

    async fn fixture(url: &str) -> Option<CompanyFixture> {
        let seeded = crate::query::runner::test_support::seeded_workspace(url).await?;
        let (role_id, application_id): (Uuid, Uuid) = sqlx::query_as(
            "SELECT id, \"applicationId\" FROM core.role WHERE \"workspaceId\" = $1 ORDER BY id LIMIT 1",
        )
        .bind(seeded.workspace_id)
        .fetch_one(&seeded.state.core_pool)
        .await
        .expect("loading fixture role");
        let company_object_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM core.\"objectMetadata\" WHERE \"workspaceId\" = $1 AND \"nameSingular\" = $2",
        )
        .bind(seeded.workspace_id)
        .bind(COMPANY_OBJECT_NAME)
        .fetch_one(&seeded.state.core_pool)
        .await
        .expect("loading company object metadata");
        let category_field_id = Uuid::new_v4();
        let category_options = json!([
            { "id": "acme", "value": "Acme", "label": "Acme", "position": 0, "color": "gray" },
            { "id": "beta", "value": "Beta", "label": "Beta", "position": 1, "color": "gray" },
            { "id": "other", "value": "Other", "label": "Other", "position": 2, "color": "gray" },
        ]);
        sqlx::query(
            "INSERT INTO core.\"fieldMetadata\" \
             (id, \"objectMetadataId\", type, name, label, \"defaultValue\", description, icon, \
              options, settings, \"isActive\", \"isSystem\", \"isUIReadOnly\", \"isNullable\", \
              \"workspaceId\", \"isLabelSyncedWithName\", \"relationTargetFieldMetadataId\", \
              \"relationTargetObjectMetadataId\", \"morphId\", \"universalIdentifier\", \
              \"applicationId\", \"isUIEditable\", \"isSystemSideEffect\", overrides, \
              writability, \"isAuditLogged\", \"disabledFieldPermissionActions\") \
             VALUES ($1, $2, 'SELECT', 'category', 'Category', NULL, NULL, NULL, $3, NULL, \
                     true, false, false, true, $4, false, NULL, NULL, NULL, $5, $6, true, \
                     false, NULL, 'OPEN', true, NULL)",
        )
        .bind(category_field_id)
        .bind(company_object_id)
        .bind(&category_options)
        .bind(seeded.workspace_id)
        .bind(Uuid::new_v4())
        .bind(application_id)
        .execute(&seeded.state.core_pool)
        .await
        .expect("inserting company select field");
        let maps = crate::metadata::flat::load_workspace_flat_maps(
            &seeded.state.core_pool,
            seeded.workspace_id,
        )
        .await
        .expect("loading company fixture maps");
        let company = maps
            .object_metadata_maps
            .by_universal_identifier
            .values()
            .find(|object| object.name_singular == COMPANY_OBJECT_NAME)
            .cloned()
            .expect("company metadata");
        sqlx::query(&format!(
            "ALTER TABLE {}.{} ADD COLUMN IF NOT EXISTS \"category\" text",
            quote_identifier(&nudge_core::ids::workspace_schema_name(
                &seeded.workspace_id
            ))
            .unwrap(),
            quote_identifier(&company_target_table(&company)).unwrap()
        ))
        .execute(&seeded.state.core_pool)
        .await
        .expect("adding company select column");
        let name_field_id = company
            .field_ids
            .iter()
            .find_map(|field_id| {
                let field = maps.field_metadata_maps.find_by_id(field_id)?;
                (field.name == "name").then_some(field.id)
            })
            .expect("company name field");
        let category_field_id = company
            .field_ids
            .iter()
            .find_map(|field_id| {
                let field = maps.field_metadata_maps.find_by_id(field_id)?;
                (field.name == "category").then_some(field.id)
            })
            .expect("company category field");
        let user_id = Uuid::new_v4();
        sqlx::query("INSERT INTO core.user (id, email) VALUES ($1, $2)")
            .bind(user_id)
            .bind(format!("member-{}@example.test", Uuid::new_v4()))
            .execute(&seeded.state.core_pool)
            .await
            .expect("inserting fixture user");
        let user_workspace_id: Uuid = sqlx::query_scalar(
            "INSERT INTO core.\"userWorkspace\" (\"userId\", \"workspaceId\") VALUES ($1, $2) RETURNING id",
        )
        .bind(user_id)
        .bind(seeded.workspace_id)
        .fetch_one(&seeded.state.core_pool)
        .await
        .expect("inserting fixture membership");
        sqlx::query(
            "INSERT INTO core.\"roleTarget\" \
             (\"workspaceId\", \"roleId\", \"userWorkspaceId\", \"universalIdentifier\", \"applicationId\") \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(seeded.workspace_id)
        .bind(role_id)
        .bind(user_workspace_id)
        .bind(Uuid::new_v4())
        .bind(application_id)
        .execute(&seeded.state.core_pool)
        .await
        .expect("inserting fixture role target");
        let runner = build_runner_schema(&seeded.workspace_id, &maps).expect("fixture runner");
        let member_model = runner
            .object(WORKSPACE_MEMBER_OBJECT_NAME)
            .expect("workspace member model");
        let member_id: Uuid = sqlx::query_scalar(&format!(
            "INSERT INTO {}.{} (\"userId\", \"userEmail\") VALUES ($1, $2) RETURNING id",
            quote_identifier(&member_model.shape.schema_name).unwrap(),
            quote_identifier(&member_model.shape.table_name).unwrap()
        ))
        .bind(user_id)
        .bind(format!("member-{}@example.test", user_id.simple()))
        .fetch_one(&seeded.state.core_pool)
        .await
        .expect("inserting fixture workspace member");
        Some(CompanyFixture {
            seeded,
            maps,
            role_id,
            application_id,
            company_id: company.id,
            name_field_id,
            category_field_id,
            member_id,
            user_id,
            user_workspace_id,
        })
    }

    impl CompanyFixture {
        fn company_table(&self) -> String {
            let company = self
                .maps
                .object_metadata_maps
                .by_universal_identifier
                .values()
                .find(|object| object.id == self.company_id)
                .expect("company metadata");
            format!(
                "{}.{}",
                quote_identifier(&nudge_core::ids::workspace_schema_name(
                    &self.seeded.workspace_id
                ))
                .unwrap(),
                quote_identifier(&company_target_table(company)).unwrap()
            )
        }

        fn auth(&self) -> AuthContext {
            AuthContext {
                token_type: TokenType::Access,
                user_id: Some(self.user_id),
                workspace_id: Some(self.seeded.workspace_id),
                user_workspace_id: Some(self.user_workspace_id),
                workspace_member_id: Some(self.member_id),
                api_key_id: None,
                agent_id: None,
                auth_provider: None,
                is_impersonating: false,
            }
        }

        async fn set_feature(&self, key: &str, value: bool) {
            sqlx::query(
                "INSERT INTO core.\"featureFlag\" (key, \"workspaceId\", value) VALUES ($1, $2, $3) \
                 ON CONFLICT (\"workspaceId\", key) DO UPDATE SET value = EXCLUDED.value",
            )
            .bind(key)
            .bind(self.seeded.workspace_id)
            .bind(value)
            .execute(&self.seeded.state.core_pool)
            .await
            .expect("setting feature flag");
            self.seeded
                .state
                .permission_cache
                .invalidate_workspace(self.seeded.workspace_id);
        }

        async fn clear_rules(&self) {
            sqlx::query(
                "DELETE FROM core.\"rowLevelPermissionPredicate\" WHERE \"roleId\" = $1 AND \"objectMetadataId\" = $2",
            )
            .bind(self.role_id)
            .bind(self.company_id)
            .execute(&self.seeded.state.core_pool)
            .await
            .expect("clearing predicates");
            sqlx::query(
                "DELETE FROM core.\"rowLevelPermissionRule\" WHERE \"roleId\" = $1 AND \"objectMetadataId\" = $2",
            )
            .bind(self.role_id)
            .bind(self.company_id)
            .execute(&self.seeded.state.core_pool)
            .await
            .expect("clearing rules");
        }

        async fn set_rule(
            &self,
            value: Value,
            member_field_id: Option<Uuid>,
            relation_field_id: Option<Uuid>,
            target_field_id: Option<Uuid>,
        ) {
            self.set_rule_with_operand(
                value,
                "IS",
                member_field_id,
                relation_field_id,
                target_field_id,
            )
            .await;
        }

        async fn set_rule_with_operand(
            &self,
            value: Value,
            operand: &str,
            member_field_id: Option<Uuid>,
            relation_field_id: Option<Uuid>,
            target_field_id: Option<Uuid>,
        ) {
            self.clear_rules().await;
            sqlx::query("UPDATE core.role SET \"canReadAllObjectRecords\" = false WHERE id = $1")
                .bind(self.role_id)
                .execute(&self.seeded.state.core_pool)
                .await
                .expect("restricting role reads");
            let rule_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO core.\"rowLevelPermissionRule\" \
                 (\"workspaceId\", \"universalIdentifier\", \"applicationId\", id, \"roleId\", \
                  \"objectMetadataId\", \"canReadRecords\", \"canCreateRecords\", \"canUpdateRecords\", \
                  \"canSoftDeleteRecords\", \"canDestroyRecords\") \
                 VALUES ($1, $2, $3, $4, $5, $6, true, true, true, true, false)",
            )
            .bind(self.seeded.workspace_id)
            .bind(Uuid::new_v4())
            .bind(self.application_id)
            .bind(rule_id)
            .bind(self.role_id)
            .bind(self.company_id)
            .execute(&self.seeded.state.core_pool)
            .await
            .expect("inserting company rule");
            sqlx::query(
                "INSERT INTO core.\"rowLevelPermissionPredicate\" \
                 (\"universalIdentifier\", \"applicationId\", \"fieldMetadataId\", \"objectMetadataId\", \
                  operand, value, \"workspaceId\", \"roleId\", \"rowLevelPermissionRuleId\", \
                  \"workspaceMemberFieldMetadataId\", \"relationFieldMetadataId\") \
                 VALUES ($1, $2, $3, $4, $5::text::core.\"rowLevelPermissionPredicate_operand_enum\", $6::jsonb, $7, $8, $9, $10, $11)",
            )
            .bind(Uuid::new_v4())
            .bind(self.application_id)
            .bind(target_field_id.unwrap_or(self.name_field_id))
            .bind(self.company_id)
            .bind(operand)
            .bind(value)
            .bind(self.seeded.workspace_id)
            .bind(self.role_id)
            .bind(rule_id)
            .bind(member_field_id)
            .bind(relation_field_id)
            .execute(&self.seeded.state.core_pool)
            .await
            .expect("inserting company predicate");
            self.seeded
                .state
                .permission_cache
                .invalidate_workspace(self.seeded.workspace_id);
        }

        async fn permissions(&self) -> RoleRecordsPermissions {
            crate::permissions::settings::records_permissions_for_auth(
                &self.seeded.state,
                &self.auth(),
            )
            .await
            .expect("loading company permissions")
            .expect("company permissions")
        }

        async fn runner(&self) -> RunnerSchema {
            let permissions = self.permissions().await;
            RunnerSchema::build_with_permissions(
                &self.seeded.workspace_id,
                &self
                    .maps
                    .object_metadata_maps
                    .by_universal_identifier
                    .values()
                    .cloned()
                    .collect::<Vec<_>>(),
                &self.maps.field_metadata_maps,
                &self.maps.index_metadata_maps,
                permissions,
                Some(self.member_id),
            )
            .expect("company permission runner")
        }

        async fn insert_company(&self, name: &str) -> Uuid {
            sqlx::query_scalar(&format!(
                "INSERT INTO {} (name) VALUES ($1) RETURNING id",
                self.company_table()
            ))
            .bind(name)
            .fetch_one(&self.seeded.state.core_pool)
            .await
            .expect("inserting company")
        }

        async fn insert_company_category(&self, name: &str, category: &str) -> Uuid {
            sqlx::query_scalar(&format!(
                "INSERT INTO {} (name, \"category\") VALUES ($1, $2) RETURNING id",
                self.company_table()
            ))
            .bind(name)
            .bind(category)
            .fetch_one(&self.seeded.state.core_pool)
            .await
            .expect("inserting company with category")
        }

        async fn array(&self, company_id: Uuid) -> Vec<i32> {
            sqlx::query_scalar(&format!(
                "SELECT {} FROM {} WHERE id = $1",
                quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME).unwrap(),
                self.company_table()
            ))
            .bind(company_id)
            .fetch_one(&self.seeded.state.core_pool)
            .await
            .expect("reading company access groups")
        }

        async fn group_types(&self) -> Vec<String> {
            sqlx::query_scalar(&format!(
                "SELECT type::text FROM {}.{} WHERE \"objectMetadataId\" = $1 ORDER BY \"compactId\"",
                quote_identifier(&self.seeded.schema_name).unwrap(),
                quote_identifier("accessGroup").unwrap()
            ))
            .bind(self.company_id)
            .fetch_all(&self.seeded.state.core_pool)
            .await
            .expect("reading access group types")
        }
    }

    #[tokio::test]
    async fn materialization_overlap_fail_closed_and_mutation_recompute() {
        let Some(url) = std::env::var("NUDGE_TEST_DATABASE_URL").ok() else {
            return;
        };
        let _guard = DB_TEST_LOCK.lock().await;
        let fixture = fixture(&url).await.expect("company fixture");
        fixture.set_feature("IS_ACCESS_GROUPS_ENABLED", true).await;
        fixture
            .set_rule(
                json!(["Acme", "Beta"]),
                None,
                None,
                Some(fixture.category_field_id),
            )
            .await;
        let acme = fixture.insert_company_category("Acme", "Acme").await;
        let beta = fixture.insert_company_category("Beta", "Beta").await;
        let other = fixture.insert_company_category("Other", "Other").await;

        let permissions = fixture.permissions().await;
        let compact_ids = permissions
            .company_access_group_ids
            .as_ref()
            .expect("materialized compact ids");
        assert_eq!(compact_ids.len(), 2);
        assert_eq!(
            fixture.group_types().await,
            vec!["RLS_LITERAL_VALUE", "RLS_LITERAL_VALUE"]
        );
        assert_eq!(fixture.array(acme).await.len(), 1);
        assert_eq!(fixture.array(beta).await.len(), 1);
        assert_ne!(fixture.array(acme).await, fixture.array(beta).await);
        assert!(fixture.array(other).await.is_empty());

        let runner = fixture.runner().await;
        let filter = crate::query::runner::rls::row_access_filter(
            &fixture.seeded.state.core_pool,
            &runner,
            COMPANY_OBJECT_NAME,
            ObjectRecordsOperationType::Read,
            &mut Suffixes::new(),
        )
        .await
        .expect("company read filter")
        .expect("company row filter");
        assert!(
            filter.render().contains("_accessGroupIds\" &&"),
            "render: {}",
            filter.render()
        );
        let result = crate::query::runner::find_many(
            &fixture.seeded.state.core_pool,
            &runner,
            COMPANY_OBJECT_NAME,
            &FindManyArgs {
                selected_field_names: vec![String::from("id"), String::from("name")],
                ..Default::default()
            },
            false,
            &[],
        )
        .await
        .expect("filtered companies");
        let mut names = result
            .records
            .iter()
            .filter_map(|record| record.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>();
        names.sort_unstable();
        assert_eq!(names, vec!["Acme", "Beta"]);

        sqlx::query(&format!(
            "UPDATE {} SET {} = NULL WHERE id = $1",
            fixture.company_table(),
            quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME).unwrap()
        ))
        .bind(other)
        .execute(&fixture.seeded.state.core_pool)
        .await
        .expect("invalidating company array");
        let healed = fixture.permissions().await;
        assert!(healed.company_access_group_ids.is_some());
        assert!(fixture.array(other).await.is_empty());
        let acme_before = fixture.array(acme).await;
        let beta_before = fixture.array(beta).await;
        let RebuildOutcome::Materialized(unchanged) = rebuild_company_access_groups(
            &fixture.seeded.state.core_pool,
            fixture.seeded.workspace_id,
        )
        .await
        .expect("idempotent rebuild") else {
            panic!("expected materialization");
        };
        assert_eq!(unchanged.group_count, 2);
        assert_eq!(unchanged.updated_record_count, 3);
        assert_eq!(fixture.array(acme).await, acme_before);
        assert_eq!(fixture.array(beta).await, beta_before);
        assert!(fixture.array(other).await.is_empty());

        let actor =
            crate::query::runner::ActorContext::manual(Some(fixture.member_id.to_string()), None);
        let created = crate::query::runner::create_one(
            &fixture.seeded.state.core_pool,
            &runner,
            COMPANY_OBJECT_NAME,
            json!({"name": "Acme", "category": "Acme"})
                .as_object()
                .unwrap()
                .clone(),
            false,
            false,
            Some(&actor),
        )
        .await
        .expect("creating grouped company");
        assert_eq!(fixture.array(created).await, fixture.array(acme).await);
        crate::query::runner::run_mutation_one(
            &fixture.seeded.state.core_pool,
            &runner,
            COMPANY_OBJECT_NAME,
            FilteredMutationKind::Update,
            Some(json!({"id": {"eq": created.to_string()}})),
            Some(json!({"category": "Beta"})),
            Some(&actor),
            false,
            vec![String::from("id")],
        )
        .await
        .expect("updating company between allowed groups");
        assert_eq!(fixture.array(created).await, fixture.array(beta).await);
        crate::query::runner::run_mutation_one(
            &fixture.seeded.state.core_pool,
            &runner,
            COMPANY_OBJECT_NAME,
            FilteredMutationKind::SoftDelete,
            Some(json!({"id": {"eq": created.to_string()}})),
            None,
            Some(&actor),
            false,
            vec![String::from("id")],
        )
        .await
        .expect("soft deleting company");
        crate::query::runner::run_mutation_one(
            &fixture.seeded.state.core_pool,
            &runner,
            COMPANY_OBJECT_NAME,
            FilteredMutationKind::Restore,
            Some(json!({"id": {"eq": created.to_string()}})),
            None,
            Some(&actor),
            true,
            vec![String::from("id")],
        )
        .await
        .expect("restoring company");
        assert_eq!(fixture.array(created).await, fixture.array(beta).await);

        crate::query::runner::test_support::drop_workspace(&fixture.seeded).await;
    }

    #[tokio::test]
    async fn read_check_fails_closed_on_generation_drift_and_dirty_row() {
        let Some(url) = std::env::var("NUDGE_TEST_DATABASE_URL").ok() else {
            return;
        };
        let _guard = DB_TEST_LOCK.lock().await;
        let fixture = fixture(&url).await.expect("fail-closed company fixture");
        fixture.set_feature("IS_ACCESS_GROUPS_ENABLED", true).await;
        fixture
            .set_rule(json!(["Acme"]), None, None, Some(fixture.category_field_id))
            .await;
        let acme = fixture.insert_company_category("Acme", "Acme").await;
        let other = fixture.insert_company_category("Other", "Other").await;

        let pool = &fixture.seeded.state.core_pool;
        let runner = fixture.runner().await;
        let state_table = format!(
            "{}.{}",
            quote_identifier(&fixture.seeded.schema_name).unwrap(),
            quote_identifier(ACCESS_GROUP_STATE_TABLE_NAME).unwrap()
        );
        let ids_column = quote_identifier(ACCESS_GROUP_IDS_COLUMN_NAME).unwrap();
        let generation_column = quote_identifier(ACCESS_GROUP_GENERATION_COLUMN_NAME).unwrap();
        let materialized_column =
            quote_identifier(ACCESS_GROUP_MATERIALIZED_GENERATION_COLUMN_NAME).unwrap();
        let permissions = fixture.permissions().await;
        let expected_groups = build_role_access_groups(
            fixture.company_id,
            permissions
                .object(fixture.company_id)
                .expect("company object permissions"),
            ObjectRecordsOperationType::Read,
            None,
            false,
        )
        .expect("company access groups");

        let state = || async {
            sqlx::query_as::<_, (i64, i32)>(&format!(
                "SELECT generation, {materialized_column} FROM {state_table} WHERE id = 1"
            ))
            .fetch_one(pool)
            .await
            .expect("reading access group state")
        };
        let dirty_rows = || async {
            sqlx::query_scalar::<_, i64>(&format!(
                "SELECT count(*) FROM {} WHERE {ids_column} IS NULL OR {generation_column} IS NULL",
                fixture.company_table()
            ))
            .fetch_one(pool)
            .await
            .expect("counting dirty company rows")
        };
        let materializable = || async {
            find_materialized_compact_ids(
                pool,
                &runner,
                fixture.seeded.workspace_id,
                fixture.company_id,
                &expected_groups,
            )
            .await
            .expect("materialization check")
        };

        assert!(materializable().await.is_some(), "fresh sweep is readable");
        let (generation, materialized) = state().await;
        assert_eq!(generation, i64::from(materialized));
        assert_eq!(dirty_rows().await, 0);

        let company_model = runner.object(COMPANY_OBJECT_NAME).expect("company model");
        let company_schema_name = company_model.shape.schema_name.clone();
        let company_table_name = company_model.shape.table_name.clone();
        let probe_sql = company_dirty_probe_sql(
            &company_schema_name,
            &company_table_name,
            &format!("(SELECT generation = {materialized_column} FROM {state_table} WHERE id = 1)"),
        )
        .expect("company dirty probe");
        let index_def: Option<String> = sqlx::query_scalar(
            "SELECT indexdef FROM pg_indexes WHERE schemaname = $1 AND indexname = $2",
        )
        .bind(&fixture.seeded.schema_name)
        .bind(ACCESS_GROUP_DIRTY_INDEX_NAME)
        .fetch_optional(pool)
        .await
        .expect("reading dirty index");
        let index_def = index_def.expect("dirty row partial index");
        assert!(
            index_def.contains("\"_accessGroupIds\" IS NULL")
                && index_def.contains("\"_accessGroupGeneration\" IS NULL")
                && index_def.contains("WHERE"),
            "dirty index definition: {index_def}"
        );
        let mut connection = pool.acquire().await.expect("probe plan connection");
        sqlx::query("SET enable_seqscan = off")
            .execute(&mut *connection)
            .await
            .expect("disabling sequential scans");
        let plan: Vec<String> = sqlx::query_scalar(&format!("EXPLAIN (COSTS OFF) {probe_sql}"))
            .fetch_all(&mut *connection)
            .await
            .expect("planning dirty probe");
        sqlx::query("RESET enable_seqscan")
            .execute(&mut *connection)
            .await
            .expect("restoring sequential scans");
        assert!(
            plan.iter()
                .any(|line| line.contains(ACCESS_GROUP_DIRTY_INDEX_NAME)),
            "dirty probe plan: {}",
            plan.join("\n")
        );

        sqlx::query(&format!(
            "UPDATE {state_table} SET generation = generation + 1 WHERE id = 1"
        ))
        .execute(pool)
        .await
        .expect("advancing state generation");
        assert_eq!(dirty_rows().await, 0);
        assert!(
            materializable().await.is_none(),
            "generation past materializedGeneration must fail closed"
        );
        let healed = fixture.permissions().await;
        assert!(
            healed.company_access_group_ids.is_some(),
            "generation drift must heal through a rebuild"
        );
        let (generation, materialized) = state().await;
        assert_eq!(generation, i64::from(materialized));
        assert!(materializable().await.is_some());

        sqlx::query(&format!(
            "UPDATE {} SET name = 'Renamed' WHERE id = $1",
            fixture.company_table()
        ))
        .bind(other)
        .execute(pool)
        .await
        .expect("writing a company row out of band");
        let (generation, materialized) = state().await;
        assert_eq!(
            generation,
            i64::from(materialized),
            "an out of band company write leaves the state aligned"
        );
        assert_eq!(dirty_rows().await, 1);
        assert!(
            materializable().await.is_none(),
            "a dirty company row must fail closed"
        );
        let healed = fixture.permissions().await;
        assert!(
            healed.company_access_group_ids.is_some(),
            "a dirty row must heal through a rebuild"
        );
        assert!(fixture.array(other).await.is_empty());
        assert_eq!(fixture.array(acme).await.len(), 1);
        assert_eq!(dirty_rows().await, 0);
        let stale_generation: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {} WHERE {generation_column} IS DISTINCT FROM \
             (SELECT generation FROM {state_table} WHERE id = 1)",
            fixture.company_table()
        ))
        .fetch_one(pool)
        .await
        .expect("counting stale generations");
        assert_eq!(stale_generation, 0);
        assert!(materializable().await.is_some());

        crate::query::runner::test_support::drop_workspace(&fixture.seeded).await;
    }

    #[tokio::test]
    async fn member_values_materialize_and_stale_groups_are_removed() {
        let Some(url) = std::env::var("NUDGE_TEST_DATABASE_URL").ok() else {
            return;
        };
        let _guard = DB_TEST_LOCK.lock().await;
        let fixture = fixture(&url).await.expect("member company fixture");
        fixture.set_feature("IS_ACCESS_GROUPS_ENABLED", true).await;
        let member_object = fixture
            .maps
            .object_metadata_maps
            .by_universal_identifier
            .values()
            .find(|object| object.name_singular == WORKSPACE_MEMBER_OBJECT_NAME)
            .expect("workspace member metadata");
        let member_field_id = member_object
            .field_ids
            .iter()
            .find_map(|field_id| {
                let field = fixture.maps.field_metadata_maps.find_by_id(field_id)?;
                (field.name == "userEmail").then_some(field.id)
            })
            .expect("workspace member email field");
        let email: String = sqlx::query_scalar(&format!(
            "SELECT \"userEmail\" FROM {}.{} WHERE id = $1",
            quote_identifier(&fixture.seeded.schema_name).unwrap(),
            quote_identifier(&company_target_table(member_object)).unwrap()
        ))
        .bind(fixture.member_id)
        .fetch_one(&fixture.seeded.state.core_pool)
        .await
        .expect("reading member email");
        fixture
            .set_rule(Value::Null, Some(member_field_id), None, None)
            .await;
        let matching = fixture.insert_company(&email).await;
        let other = fixture.insert_company("Other").await;
        let permissions = fixture.permissions().await;
        assert!(permissions.company_access_group_ids.is_some());
        assert_eq!(fixture.group_types().await, vec!["RLS_MEMBER_VALUE"]);
        assert_eq!(fixture.array(matching).await.len(), 1);
        assert!(fixture.array(other).await.is_empty());

        sqlx::query(&format!(
            "UPDATE {}.{} SET \"userEmail\" = 'changed@example.test' WHERE id = $1",
            quote_identifier(&fixture.seeded.schema_name).unwrap(),
            quote_identifier(&company_target_table(member_object)).unwrap()
        ))
        .bind(fixture.member_id)
        .execute(&fixture.seeded.state.core_pool)
        .await
        .expect("changing member value");
        rebuild_company_access_groups(&fixture.seeded.state.core_pool, fixture.seeded.workspace_id)
            .await
            .expect("rebuilding after member change");
        let changed = fixture.insert_company("changed@example.test").await;
        rebuild_company_access_groups(&fixture.seeded.state.core_pool, fixture.seeded.workspace_id)
            .await
            .expect("recomputing new company");
        assert_eq!(fixture.group_types().await, vec!["RLS_MEMBER_VALUE"]);
        assert!(fixture.array(matching).await.is_empty());
        assert!(fixture.array(other).await.is_empty());
        assert_eq!(fixture.array(changed).await.len(), 1);
        let group_count: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {}.{} WHERE \"objectMetadataId\" = $1",
            quote_identifier(&fixture.seeded.schema_name).unwrap(),
            quote_identifier("accessGroup").unwrap()
        ))
        .bind(fixture.company_id)
        .fetch_one(&fixture.seeded.state.core_pool)
        .await
        .expect("counting groups");
        assert_eq!(group_count, 1);

        crate::query::runner::test_support::drop_workspace(&fixture.seeded).await;
    }

    #[tokio::test]
    async fn overlays_deny_and_relation_rules_stay_on_live_rls() {
        let Some(url) = std::env::var("NUDGE_TEST_DATABASE_URL").ok() else {
            return;
        };
        let _guard = DB_TEST_LOCK.lock().await;
        let fixture = fixture(&url).await.expect("fail-closed company fixture");
        fixture.set_feature("IS_ACCESS_GROUPS_ENABLED", true).await;
        fixture
            .set_feature("IS_PRIVATE_RECORDS_ENABLED", true)
            .await;
        sqlx::query("UPDATE core.role SET \"canReadAllObjectRecords\" = true WHERE id = $1")
            .bind(fixture.role_id)
            .execute(&fixture.seeded.state.core_pool)
            .await
            .expect("granting all records");
        fixture
            .seeded
            .state
            .permission_cache
            .invalidate_workspace(fixture.seeded.workspace_id);
        let runner = fixture.runner().await;
        let denied = crate::query::runner::find_many(
            &fixture.seeded.state.core_pool,
            &runner,
            COMPANY_OBJECT_NAME,
            &FindManyArgs::default(),
            false,
            &[],
        )
        .await
        .expect_err("private overlay must deny company reads");
        assert_eq!(denied.code(), "PERMISSION_DENIED");

        fixture
            .set_feature("IS_PRIVATE_RECORDS_ENABLED", false)
            .await;
        fixture
            .set_feature("IS_ROW_LEVEL_PERMISSION_RELATION_HOP_ENABLED", true)
            .await;
        let company = fixture
            .maps
            .object_metadata_maps
            .by_universal_identifier
            .values()
            .find(|object| object.id == fixture.company_id)
            .expect("company metadata");
        let person = fixture
            .maps
            .object_metadata_maps
            .by_universal_identifier
            .values()
            .find(|object| object.name_singular == "person")
            .expect("person metadata");
        let relation_field_id = company
            .field_ids
            .iter()
            .find_map(|field_id| {
                let field = fixture.maps.field_metadata_maps.find_by_id(field_id)?;
                (field.field_type == crate::metadata::flat::FieldMetadataType::Relation
                    && field.relation_target_object_metadata_id == Some(person.id))
                .then_some(field.id)
            })
            .expect("company person relation");
        let target_field_id = person
            .field_ids
            .iter()
            .find_map(|field_id| {
                let field = fixture.maps.field_metadata_maps.find_by_id(field_id)?;
                (field.name == "name").then_some(field.id)
            })
            .expect("person name field");
        fixture
            .set_rule_with_operand(
                json!("Acme"),
                "CONTAINS",
                None,
                Some(relation_field_id),
                Some(target_field_id),
            )
            .await;
        let permissions = fixture.permissions().await;
        assert!(permissions.company_access_group_ids.is_none());
        let runner = RunnerSchema::build_with_permissions(
            &fixture.seeded.workspace_id,
            &fixture
                .maps
                .object_metadata_maps
                .by_universal_identifier
                .values()
                .cloned()
                .collect::<Vec<_>>(),
            &fixture.maps.field_metadata_maps,
            &fixture.maps.index_metadata_maps,
            permissions,
            Some(fixture.member_id),
        )
        .expect("relation rule runner");
        let filter = crate::query::runner::rls::row_access_filter(
            &fixture.seeded.state.core_pool,
            &runner,
            COMPANY_OBJECT_NAME,
            ObjectRecordsOperationType::Read,
            &mut Suffixes::new(),
        )
        .await
        .expect("live relation filter")
        .expect("relation row filter");
        assert!(filter.render().contains("EXISTS"));
        assert!(!filter.render().contains("_accessGroupIds"));

        crate::query::runner::test_support::drop_workspace(&fixture.seeded).await;
    }
}
