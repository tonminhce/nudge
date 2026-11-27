use std::collections::BTreeMap;
use std::collections::HashSet;

use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

use crate::auth::middleware::AuthContext;
use crate::metadata::api::service::load_flat_maps;
use crate::state::AppState;

use super::context::PermissionsContext;
use super::error::PermissionError;
use super::error::PermissionErrorCode;
use super::kernel::compute_context_records_permissions;
use super::loader::load_workspace_permission_set;
use super::model::RoleDefinition;
use super::model::RoleRecordsPermissions;
use super::model::WorkspacePermissionSet;
use super::model::ALL_PERMISSION_FLAGS;
use super::model::CREATE_RECORDS_PERMISSION_ENABLED;
use super::model::TOOL_PERMISSION_FLAGS;

pub fn has_permission_flag(role: &RoleDefinition, key: &str) -> bool {
    role.permission_flags.contains(key)
        || if TOOL_PERMISSION_FLAGS.contains(&key) {
            role.can_access_all_tools
        } else {
            role.can_update_all_settings
        }
}

pub(crate) fn effective_permission_flags(role: &RoleDefinition) -> HashSet<String> {
    ALL_PERMISSION_FLAGS
        .iter()
        .filter(|key| has_permission_flag(role, key))
        .map(|key| (*key).to_string())
        .collect()
}

pub async fn user_workspace_permissions_for_auth(
    state: &AppState,
    auth: &AuthContext,
) -> Result<Option<Value>, PermissionError> {
    let Some((context, workspace)) = load_context_and_set(state, auth).await? else {
        return Ok(None);
    };
    Ok(Some(user_workspace_permissions(&workspace, &context)))
}

pub async fn records_permissions_for_auth(
    state: &AppState,
    auth: &AuthContext,
) -> Result<Option<RoleRecordsPermissions>, PermissionError> {
    let Some((context, workspace)) = load_context_and_set(state, auth).await? else {
        return Ok(None);
    };
    Ok(Some(compute_context_records_permissions(
        &workspace, &context,
    )))
}

pub async fn current_user_workspace_permissions_for_auth(
    state: &AppState,
    auth: &AuthContext,
) -> Result<Option<Value>, PermissionError> {
    let Some((context, workspace)) = load_context_and_set(state, auth).await? else {
        return Ok(None);
    };
    Ok(Some(current_user_workspace_permissions(
        &workspace,
        &context,
        auth.user_workspace_id,
    )))
}

async fn load_context_and_set(
    state: &AppState,
    auth: &AuthContext,
) -> Result<Option<(PermissionsContext, WorkspacePermissionSet)>, PermissionError> {
    let Some(workspace_id) = auth.workspace_id else {
        return Ok(None);
    };
    let context = PermissionsContext::load(&state.core_pool, auth).await?;
    let maps = load_flat_maps(state, workspace_id).await.map_err(|error| {
        PermissionError::new(
            PermissionErrorCode::RecordPermissionQueryFailed,
            error.message,
        )
    })?;
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
    let workspace = load_workspace_permission_set(
        &state.core_pool,
        workspace_id,
        &objects,
        &fields,
        &state.permission_cache,
    )
    .await?;
    Ok(Some((context, workspace)))
}

pub fn user_workspace_permissions(
    workspace: &WorkspacePermissionSet,
    context: &PermissionsContext,
) -> Value {
    let flags = permission_flags(workspace, context);
    let mut records_permissions = compute_context_records_permissions(workspace, context);
    if !CREATE_RECORDS_PERMISSION_ENABLED {
        follow_update_when_create_permission_is_disabled(&mut records_permissions);
    }
    let readable_objects = records_permissions
        .objects
        .iter()
        .filter(|(_, object)| object.can_read_some_records)
        .filter_map(|(id, _)| Uuid::parse_str(id).ok())
        .collect::<Vec<_>>();
    json!({
        "permissionFlags": flags,
        "recordsPermissions": records_permissions,
        "objectMetadataIdsWithReadableRecords": readable_objects,
    })
}

pub fn current_user_workspace_permissions(
    workspace: &WorkspacePermissionSet,
    context: &PermissionsContext,
    user_workspace_id: Option<Uuid>,
) -> Value {
    let flags = permission_flags(workspace, context)
        .into_iter()
        .filter(|(_, granted)| *granted)
        .map(|(key, _)| key)
        .collect::<Vec<_>>();
    let mut records_permissions = compute_context_records_permissions(workspace, context);
    if !CREATE_RECORDS_PERMISSION_ENABLED {
        follow_update_when_create_permission_is_disabled(&mut records_permissions);
    }
    let object_records_permissions = ordered_object_permissions(workspace, records_permissions);
    json!({
        "id": user_workspace_id,
        "permissionFlags": flags,
        "objectRecordsPermissions": object_records_permissions,
    })
}

fn ordered_object_permissions(
    workspace: &WorkspacePermissionSet,
    mut permissions: RoleRecordsPermissions,
) -> Vec<super::model::ObjectRecordsPermissions> {
    let order = if workspace.object_order.is_empty() {
        workspace.object_ids.iter().copied().collect::<Vec<_>>()
    } else {
        workspace.object_order.clone()
    };
    let mut ordered = order
        .iter()
        .filter_map(|id| permissions.objects.remove(&id.to_string()))
        .collect::<Vec<_>>();
    ordered.extend(permissions.objects.into_values());
    ordered
}

fn permission_flags(
    workspace: &WorkspacePermissionSet,
    context: &PermissionsContext,
) -> BTreeMap<String, bool> {
    let mut main_role_ids = context.main_role_ids.clone();
    if main_role_ids.is_empty() {
        main_role_ids = context.role_ids().into_iter().collect();
    }
    ALL_PERMISSION_FLAGS
        .iter()
        .map(|key| {
            let granted = if context.is_system {
                true
            } else {
                let main_grants = main_role_ids.iter().any(|role_id| {
                    workspace
                        .flags(*role_id)
                        .is_some_and(|flags| flags.contains(*key))
                });
                let limiting_grants = context.limiting_role_ids.iter().all(|role_id| {
                    workspace
                        .flags(*role_id)
                        .is_some_and(|flags| flags.contains(*key))
                });
                main_grants && limiting_grants
            };
            ((*key).to_string(), granted)
        })
        .collect()
}

fn follow_update_when_create_permission_is_disabled(permissions: &mut RoleRecordsPermissions) {
    for object in permissions.objects.values_mut() {
        object.can_create_all_records = object.can_update_all_records;
        if let Some(rules) = object.row_level_records_permissions.as_mut() {
            for rule in rules {
                rule.can_create_records = rule.can_update_records;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::ObjectRecordsPermissions;
    use super::*;

    fn role() -> RoleDefinition {
        RoleDefinition::empty(Uuid::new_v4())
    }

    #[test]
    fn settings_access_grants_non_tool_flags_but_not_tools() {
        let mut role = role();
        role.can_update_all_settings = true;
        assert!(has_permission_flag(&role, "DATA_MODEL"));
        assert!(has_permission_flag(&role, "ROLES"));
        assert!(!has_permission_flag(&role, "HTTP_REQUEST_TOOL"));
        role.can_access_all_tools = true;
        assert!(has_permission_flag(&role, "HTTP_REQUEST_TOOL"));
        assert!(has_permission_flag(&role, "SEND_EMAIL_TOOL"));
    }

    #[test]
    fn current_user_payload_aliases_create_to_update() {
        let object_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let workspace = WorkspacePermissionSet {
            roles: [(
                role_id,
                RoleRecordsPermissions {
                    objects: [(
                        object_id.to_string(),
                        ObjectRecordsPermissions {
                            can_update_all_records: true,
                            can_create_all_records: false,
                            can_read_some_records: true,
                            ..Default::default()
                        },
                    )]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            permission_flags: [(role_id, HashSet::new())].into_iter().collect(),
            object_ids: [object_id].into_iter().collect(),
            object_order: vec![object_id],
            ..Default::default()
        };
        let context = PermissionsContext {
            workspace_id: Some(Uuid::new_v4()),
            main_role_ids: vec![role_id],
            ..PermissionsContext::system(None)
        };
        let payload = user_workspace_permissions(&workspace, &context);
        assert_eq!(
            payload["permissionFlags"].as_object().unwrap().len(),
            ALL_PERMISSION_FLAGS.len()
        );
        assert_eq!(
            payload["recordsPermissions"]["objects"][object_id.to_string()]["canCreateAllRecords"],
            true
        );
        assert_eq!(
            payload["objectMetadataIdsWithReadableRecords"][0],
            object_id.to_string()
        );
    }

    #[test]
    fn system_payload_grants_every_flag() {
        let workspace = WorkspacePermissionSet::default();
        let context = PermissionsContext::system(Some(Uuid::new_v4()));
        let payload = user_workspace_permissions(&workspace, &context);
        assert!(payload["permissionFlags"]
            .as_object()
            .unwrap()
            .values()
            .all(|value| value.as_bool() == Some(true)));
        assert!(payload["recordsPermissions"]["canReadSomeRecords"]
            .as_bool()
            .unwrap());
    }
}
