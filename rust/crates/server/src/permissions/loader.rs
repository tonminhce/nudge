use std::collections::HashMap;

use sqlx::FromRow;
use sqlx::PgPool;
use uuid::Uuid;

use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::FlatObjectMetadata;

use super::error::PermissionError;
use super::error::PermissionErrorCode;
use super::kernel::compute_role_records_permissions;
use super::model::FieldPermissionDefinition;
use super::model::ObjectPermissionDefinition;
use super::model::RoleDefinition;
use super::model::RowLevelPermissionPredicateDefinition;
use super::model::RowLevelPermissionPredicateGroupDefinition;
use super::model::RowLevelPermissionRuleDefinition;
use super::model::WorkspacePermissionSet;
use super::settings::effective_permission_flags;
use super::PermissionCache;

#[derive(Debug, FromRow)]
struct RoleRow {
    id: Uuid,
    #[sqlx(rename = "canUpdateAllSettings")]
    can_update_all_settings: bool,
    #[sqlx(rename = "canAccessAllTools")]
    can_access_all_tools: bool,
    #[sqlx(rename = "canReadAllObjectRecords")]
    can_read_all_object_records: bool,
    #[sqlx(rename = "canUpdateAllObjectRecords")]
    can_update_all_object_records: bool,
    #[sqlx(rename = "canSoftDeleteAllObjectRecords")]
    can_soft_delete_all_object_records: bool,
    #[sqlx(rename = "canDestroyAllObjectRecords")]
    can_destroy_all_object_records: bool,
    #[sqlx(rename = "canCreateAllObjectRecords")]
    can_create_all_object_records: bool,
}

#[derive(Debug, FromRow)]
struct ObjectPermissionRow {
    #[sqlx(rename = "roleId")]
    role_id: Uuid,
    #[sqlx(rename = "objectMetadataId")]
    object_metadata_id: Uuid,
    #[sqlx(rename = "canReadObjectRecords")]
    can_read_object_records: Option<bool>,
    #[sqlx(rename = "canUpdateObjectRecords")]
    can_update_object_records: Option<bool>,
    #[sqlx(rename = "canSoftDeleteObjectRecords")]
    can_soft_delete_object_records: Option<bool>,
    #[sqlx(rename = "canDestroyObjectRecords")]
    can_destroy_object_records: Option<bool>,
    #[sqlx(rename = "canCreateObjectRecords")]
    can_create_object_records: Option<bool>,
}

#[derive(Debug, FromRow)]
struct FieldPermissionRow {
    #[sqlx(rename = "roleId")]
    role_id: Uuid,
    #[sqlx(rename = "objectMetadataId")]
    object_metadata_id: Uuid,
    #[sqlx(rename = "fieldMetadataId")]
    field_metadata_id: Uuid,
    #[sqlx(rename = "canReadFieldValue")]
    can_read_field_value: Option<bool>,
    #[sqlx(rename = "canUpdateFieldValue")]
    can_update_field_value: Option<bool>,
    #[sqlx(rename = "rowLevelPermissionRuleId")]
    row_level_permission_rule_id: Option<Uuid>,
}

#[derive(Debug, FromRow)]
struct RowLevelPermissionRuleRow {
    id: Uuid,
    #[sqlx(rename = "roleId")]
    role_id: Uuid,
    #[sqlx(rename = "objectMetadataId")]
    object_metadata_id: Uuid,
    #[sqlx(rename = "canReadRecords")]
    can_read_records: bool,
    #[sqlx(rename = "canCreateRecords")]
    can_create_records: bool,
    #[sqlx(rename = "canUpdateRecords")]
    can_update_records: bool,
    #[sqlx(rename = "canSoftDeleteRecords")]
    can_soft_delete_records: bool,
    #[sqlx(rename = "canDestroyRecords")]
    can_destroy_records: bool,
}

#[derive(Debug, FromRow)]
struct RowLevelPermissionPredicateRow {
    id: Uuid,
    #[sqlx(rename = "fieldMetadataId")]
    field_metadata_id: Uuid,
    #[sqlx(rename = "relationFieldMetadataId")]
    relation_field_metadata_id: Option<Uuid>,
    #[sqlx(rename = "objectMetadataId")]
    object_metadata_id: Uuid,
    operand: String,
    #[sqlx(rename = "subFieldName")]
    sub_field_name: Option<String>,
    #[sqlx(rename = "workspaceMemberFieldMetadataId")]
    workspace_member_field_metadata_id: Option<Uuid>,
    #[sqlx(rename = "workspaceMemberSubFieldName")]
    workspace_member_sub_field_name: Option<String>,
    #[sqlx(rename = "workspaceMemberRelationFieldMetadataId")]
    workspace_member_relation_field_metadata_id: Option<Uuid>,
    #[sqlx(rename = "rowLevelPermissionPredicateGroupId")]
    row_level_permission_predicate_group_id: Option<Uuid>,
    #[sqlx(rename = "positionInRowLevelPermissionPredicateGroup")]
    position_in_row_level_permission_predicate_group: Option<f64>,
    #[sqlx(rename = "roleId")]
    role_id: Uuid,
    value: Option<serde_json::Value>,
    #[sqlx(rename = "rowLevelPermissionRuleId")]
    row_level_permission_rule_id: Option<Uuid>,
}

#[derive(Debug, FromRow)]
struct RowLevelPermissionPredicateGroupRow {
    id: Uuid,
    #[sqlx(rename = "parentRowLevelPermissionPredicateGroupId")]
    parent_row_level_permission_predicate_group_id: Option<Uuid>,
    #[sqlx(rename = "logicalOperator")]
    logical_operator: String,
    #[sqlx(rename = "positionInRowLevelPermissionPredicateGroup")]
    position_in_row_level_permission_predicate_group: Option<f64>,
    #[sqlx(rename = "canReadRecords")]
    can_read_records: bool,
    #[sqlx(rename = "canUpdateRecords")]
    can_update_records: bool,
    #[sqlx(rename = "canSoftDeleteRecords")]
    can_soft_delete_records: bool,
    #[sqlx(rename = "canDestroyRecords")]
    can_destroy_records: bool,
    #[sqlx(rename = "roleId")]
    role_id: Uuid,
    #[sqlx(rename = "objectMetadataId")]
    object_metadata_id: Uuid,
    #[sqlx(rename = "rowLevelPermissionRuleId")]
    row_level_permission_rule_id: Option<Uuid>,
}

#[derive(Debug, FromRow)]
struct ObjectOrderRow {
    id: Uuid,
}

#[derive(Debug, FromRow)]
struct PermissionFlagRow {
    #[sqlx(rename = "roleId")]
    role_id: Uuid,
    key: String,
}

#[derive(Debug, FromRow)]
struct FeatureFlagRow {
    key: String,
    value: bool,
}

pub async fn load_workspace_permission_set(
    pool: &PgPool,
    workspace_id: Uuid,
    objects: &[FlatObjectMetadata],
    fields: &[FlatFieldMetadata],
    cache: &PermissionCache,
) -> Result<WorkspacePermissionSet, PermissionError> {
    let key = (workspace_id, cache.version(workspace_id));
    if let Some(cached) = cache.get(key) {
        return Ok(cached);
    }
    let permission_set = load_permission_set(pool, workspace_id, objects, fields).await?;
    cache.insert(key, permission_set.clone());
    Ok(permission_set)
}

async fn load_permission_set(
    pool: &PgPool,
    workspace_id: Uuid,
    objects: &[FlatObjectMetadata],
    fields: &[FlatFieldMetadata],
) -> Result<WorkspacePermissionSet, PermissionError> {
    let role_rows: Vec<RoleRow> = sqlx::query_as(
        "SELECT id, \"canUpdateAllSettings\", \"canAccessAllTools\", \
         \"canReadAllObjectRecords\", \"canUpdateAllObjectRecords\", \
         \"canSoftDeleteAllObjectRecords\", \"canDestroyAllObjectRecords\", \
         \"canCreateAllObjectRecords\" \
         FROM core.role WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    let mut roles = role_rows
        .into_iter()
        .map(|row| {
            (
                row.id,
                RoleDefinition {
                    id: row.id,
                    can_update_all_settings: row.can_update_all_settings,
                    can_access_all_tools: row.can_access_all_tools,
                    can_read_all_object_records: row.can_read_all_object_records,
                    can_update_all_object_records: row.can_update_all_object_records,
                    can_soft_delete_all_object_records: row.can_soft_delete_all_object_records,
                    can_destroy_all_object_records: row.can_destroy_all_object_records,
                    can_create_all_object_records: row.can_create_all_object_records,
                    ..RoleDefinition::empty(row.id)
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let object_rows: Vec<ObjectPermissionRow> = sqlx::query_as(
        "SELECT \"roleId\", \"objectMetadataId\", \"canReadObjectRecords\", \
         \"canUpdateObjectRecords\", \"canSoftDeleteObjectRecords\", \
         \"canDestroyObjectRecords\", \"canCreateObjectRecords\" \
         FROM core.\"objectPermission\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    for row in object_rows {
        if let Some(role) = roles.get_mut(&row.role_id) {
            role.object_permissions.insert(
                row.object_metadata_id,
                ObjectPermissionDefinition {
                    object_metadata_id: row.object_metadata_id,
                    can_read_object_records: row.can_read_object_records,
                    can_update_object_records: row.can_update_object_records,
                    can_soft_delete_object_records: row.can_soft_delete_object_records,
                    can_destroy_object_records: row.can_destroy_object_records,
                    can_create_object_records: row.can_create_object_records,
                },
            );
        }
    }

    let field_rows: Vec<FieldPermissionRow> = sqlx::query_as(
        "SELECT \"roleId\", \"objectMetadataId\", \"fieldMetadataId\", \
         \"canReadFieldValue\", \"canUpdateFieldValue\", \"rowLevelPermissionRuleId\" \
         FROM core.\"fieldPermission\" \
         WHERE \"workspaceId\" = $1 \
         AND (\"rowLevelPermissionRuleId\" IS NOT NULL \
              OR \"rowLevelPermissionPredicateGroupId\" IS NULL)",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    for row in field_rows {
        if let Some(role) = roles.get_mut(&row.role_id) {
            role.field_permissions.push(FieldPermissionDefinition {
                object_metadata_id: row.object_metadata_id,
                field_metadata_id: row.field_metadata_id,
                can_read_field_value: row.can_read_field_value,
                can_update_field_value: row.can_update_field_value,
                row_level_permission_rule_id: row.row_level_permission_rule_id,
            });
        }
    }

    let rule_rows: Vec<RowLevelPermissionRuleRow> = sqlx::query_as(
        "SELECT id, \"roleId\", \"objectMetadataId\", \"canReadRecords\", \
         \"canCreateRecords\", \"canUpdateRecords\", \"canSoftDeleteRecords\", \
         \"canDestroyRecords\" \
         FROM core.\"rowLevelPermissionRule\" WHERE \"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    for row in rule_rows {
        if let Some(role) = roles.get_mut(&row.role_id) {
            role.row_level_permission_rules
                .push(RowLevelPermissionRuleDefinition {
                    id: row.id,
                    object_metadata_id: row.object_metadata_id,
                    can_create_records: row.can_create_records,
                    can_read_records: row.can_read_records,
                    can_update_records: row.can_update_records,
                    can_soft_delete_records: row.can_soft_delete_records,
                    can_destroy_records: row.can_destroy_records,
                });
        }
    }

    let predicate_rows: Vec<RowLevelPermissionPredicateRow> = sqlx::query_as(
        "SELECT id, \"fieldMetadataId\", \"relationFieldMetadataId\", \
         \"objectMetadataId\", operand::text AS operand, \"subFieldName\", \
         \"workspaceMemberFieldMetadataId\", \"workspaceMemberSubFieldName\", \
         \"workspaceMemberRelationFieldMetadataId\", \
         \"rowLevelPermissionPredicateGroupId\", \
         \"positionInRowLevelPermissionPredicateGroup\", \"roleId\", value, \
         \"rowLevelPermissionRuleId\" \
         FROM core.\"rowLevelPermissionPredicate\" \
         WHERE \"workspaceId\" = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    for row in predicate_rows {
        if let Some(role) = roles.get_mut(&row.role_id) {
            role.row_level_permission_predicates
                .push(RowLevelPermissionPredicateDefinition {
                    id: row.id,
                    field_metadata_id: row.field_metadata_id,
                    relation_field_metadata_id: row.relation_field_metadata_id,
                    object_metadata_id: row.object_metadata_id,
                    operand: row.operand,
                    sub_field_name: row.sub_field_name,
                    workspace_member_field_metadata_id: row.workspace_member_field_metadata_id,
                    workspace_member_sub_field_name: row.workspace_member_sub_field_name,
                    workspace_member_relation_field_metadata_id: row
                        .workspace_member_relation_field_metadata_id,
                    row_level_permission_predicate_group_id: row
                        .row_level_permission_predicate_group_id,
                    position_in_row_level_permission_predicate_group: row
                        .position_in_row_level_permission_predicate_group,
                    role_id: row.role_id,
                    value: row.value,
                    row_level_permission_rule_id: row.row_level_permission_rule_id,
                });
        }
    }

    let predicate_group_rows: Vec<RowLevelPermissionPredicateGroupRow> = sqlx::query_as(
        "SELECT id, \"parentRowLevelPermissionPredicateGroupId\", \
         \"logicalOperator\"::text AS \"logicalOperator\", \
         \"positionInRowLevelPermissionPredicateGroup\", \
         \"canReadRecords\", \"canUpdateRecords\", \"canSoftDeleteRecords\", \
         \"canDestroyRecords\", \"roleId\", \"objectMetadataId\", \
         \"rowLevelPermissionRuleId\" \
         FROM core.\"rowLevelPermissionPredicateGroup\" \
         WHERE \"workspaceId\" = $1 AND \"deletedAt\" IS NULL",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    for row in predicate_group_rows {
        if let Some(role) = roles.get_mut(&row.role_id) {
            role.row_level_permission_predicate_groups.push(
                RowLevelPermissionPredicateGroupDefinition {
                    id: row.id,
                    parent_row_level_permission_predicate_group_id: row
                        .parent_row_level_permission_predicate_group_id,
                    logical_operator: row.logical_operator,
                    position_in_row_level_permission_predicate_group: row
                        .position_in_row_level_permission_predicate_group,
                    can_read_records: row.can_read_records,
                    can_update_records: row.can_update_records,
                    can_soft_delete_records: row.can_soft_delete_records,
                    can_destroy_records: row.can_destroy_records,
                    role_id: row.role_id,
                    object_metadata_id: row.object_metadata_id,
                    row_level_permission_rule_id: row.row_level_permission_rule_id,
                },
            );
        }
    }

    let flag_rows: Vec<PermissionFlagRow> = sqlx::query_as(
        "SELECT rpf.\"roleId\", pf.key \
         FROM core.\"rolePermissionFlag\" rpf \
         JOIN core.\"permissionFlag\" pf ON pf.id = rpf.\"permissionFlagId\" \
         WHERE rpf.\"workspaceId\" = $1",
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await
    .map_err(database_error)?;
    for row in flag_rows {
        if let Some(role) = roles.get_mut(&row.role_id) {
            role.permission_flags.insert(row.key);
        }
    }

    let feature_flag_rows: Vec<FeatureFlagRow> =
        sqlx::query_as("SELECT key, value FROM core.\"featureFlag\" WHERE \"workspaceId\" = $1")
            .bind(workspace_id)
            .fetch_all(pool)
            .await
            .map_err(database_error)?;
    let feature_flags = feature_flag_rows
        .into_iter()
        .map(|row| (row.key, row.value))
        .collect();

    let object_order_rows: Vec<ObjectOrderRow> =
        sqlx::query_as("SELECT id FROM core.\"objectMetadata\" WHERE \"workspaceId\" = $1")
            .bind(workspace_id)
            .fetch_all(pool)
            .await
            .map_err(database_error)?;

    let mut permission_flags = HashMap::new();
    let mut records_permissions = HashMap::new();
    for (role_id, role) in roles {
        permission_flags.insert(role_id, effective_permission_flags(&role));
        records_permissions.insert(
            role_id,
            compute_role_records_permissions(&role, objects, fields, false, None),
        );
    }
    Ok(WorkspacePermissionSet {
        roles: records_permissions,
        permission_flags,
        object_ids: objects.iter().map(|object| object.id).collect(),
        object_order: object_order_rows.into_iter().map(|row| row.id).collect(),
        feature_flags,
    })
}

fn database_error(error: sqlx::Error) -> PermissionError {
    PermissionError::new(
        PermissionErrorCode::RecordPermissionQueryFailed,
        error.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_rows_keep_role_wide_flags() {
        let row = RoleRow {
            id: Uuid::new_v4(),
            can_update_all_settings: true,
            can_access_all_tools: true,
            can_read_all_object_records: true,
            can_update_all_object_records: false,
            can_soft_delete_all_object_records: false,
            can_destroy_all_object_records: false,
            can_create_all_object_records: false,
        };
        let role = RoleDefinition {
            id: row.id,
            can_update_all_settings: row.can_update_all_settings,
            can_access_all_tools: row.can_access_all_tools,
            can_read_all_object_records: row.can_read_all_object_records,
            can_update_all_object_records: row.can_update_all_object_records,
            can_soft_delete_all_object_records: row.can_soft_delete_all_object_records,
            can_destroy_all_object_records: row.can_destroy_all_object_records,
            can_create_all_object_records: row.can_create_all_object_records,
            ..RoleDefinition::empty(row.id)
        };
        assert!(role.can_update_all_settings);
        assert!(role.can_access_all_tools);
        assert!(role.can_read_all_object_records);
    }

    #[test]
    fn effective_flags_include_explicit_and_role_wide_grants() {
        let mut role = RoleDefinition::empty(Uuid::new_v4());
        role.can_update_all_settings = true;
        role.permission_flags.insert("AI".to_string());
        let flags = effective_permission_flags(&role);
        assert!(flags.contains("DATA_MODEL"));
        assert!(flags.contains("AI"));
        assert!(!flags.contains("HTTP_REQUEST_TOOL"));
    }
}
