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
         \"canReadFieldValue\", \"canUpdateFieldValue\" \
         FROM core.\"fieldPermission\" \
         WHERE \"workspaceId\" = $1 \
         AND \"rowLevelPermissionRuleId\" IS NULL \
         AND \"rowLevelPermissionPredicateGroupId\" IS NULL",
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
                row_level_permission_rule_id: None,
            });
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
