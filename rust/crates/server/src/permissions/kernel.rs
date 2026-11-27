use std::collections::BTreeMap;
use std::collections::HashSet;

use uuid::Uuid;

use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::FlatObjectMetadata;

use super::context::PermissionsContext;
use super::model::full_access_records_permissions;
use super::model::ObjectRecordsOperationType;
use super::model::ObjectRecordsPermissions;
use super::model::RoleDefinition;
use super::model::RoleRecordsPermissions;
use super::model::RowLevelRecordsPermissions;
use super::model::WorkspacePermissionSet;
use super::restrict::restrict_disabled_actions;
use super::settings::has_permission_flag;

pub fn compute_role_records_permissions(
    role: &RoleDefinition,
    objects: &[FlatObjectMetadata],
    fields: &[FlatFieldMetadata],
    are_system_objects_read_only_by_default: bool,
    acting_application_id: Option<Uuid>,
) -> RoleRecordsPermissions {
    let mut object_permissions = BTreeMap::new();
    for object in objects {
        let override_permissions = role.object_permissions.get(&object.id);
        let is_special = matches!(
            object.name_singular.as_str(),
            "workflowRun" | "workspaceMember" | "agentChatThread"
        );
        let has_ai = has_permission_flag(role, "AI");
        let has_workflows = has_permission_flag(role, "WORKFLOWS");
        let has_workspace_members = has_permission_flag(role, "WORKSPACE_MEMBERS");
        let permission = ObjectRecordsPermissions {
            object_metadata_id: object.id.to_string(),
            can_create_all_records: resolve_object_flag(
                override_permissions.and_then(|permission| permission.can_create_object_records),
                role.can_create_all_object_records,
                object.is_system,
                ObjectRecordsOperationType::Create,
                are_system_objects_read_only_by_default,
            ) && (!is_special || has_ai),
            can_read_all_records: if object.name_singular == "workspaceMember" {
                true
            } else {
                resolve_object_flag(
                    override_permissions.and_then(|permission| permission.can_read_object_records),
                    role.can_read_all_object_records,
                    object.is_system,
                    ObjectRecordsOperationType::Read,
                    are_system_objects_read_only_by_default,
                ) && (!is_special || has_ai)
            },
            can_update_all_records: resolve_object_flag(
                override_permissions.and_then(|permission| permission.can_update_object_records),
                role.can_update_all_object_records,
                object.is_system,
                ObjectRecordsOperationType::Update,
                are_system_objects_read_only_by_default,
            ) && (!is_special || has_ai),
            can_soft_delete_all_records: resolve_object_flag(
                override_permissions
                    .and_then(|permission| permission.can_soft_delete_object_records),
                role.can_soft_delete_all_object_records,
                object.is_system,
                ObjectRecordsOperationType::SoftDelete,
                are_system_objects_read_only_by_default,
            ) && (!is_special || has_ai),
            can_destroy_all_records: resolve_object_flag(
                override_permissions.and_then(|permission| permission.can_destroy_object_records),
                role.can_destroy_all_object_records,
                object.is_system,
                ObjectRecordsOperationType::Destroy,
                are_system_objects_read_only_by_default,
            ) && (!is_special || has_ai),
            unreadable_fields: Vec::new(),
            uneditable_fields: Vec::new(),
            row_level_records_permissions: None,
            can_read_some_records: false,
        };
        let mut permission = permission;
        if object.name_singular == "workflowRun" {
            permission.can_create_all_records = has_workflows;
            permission.can_read_all_records = has_workflows;
            permission.can_update_all_records = has_workflows;
            permission.can_soft_delete_all_records = has_workflows;
            permission.can_destroy_all_records = has_workflows;
        } else if object.name_singular == "workspaceMember" {
            permission.can_create_all_records = has_workspace_members;
            permission.can_read_all_records = true;
            permission.can_update_all_records = has_workspace_members;
            permission.can_soft_delete_all_records = has_workspace_members;
            permission.can_destroy_all_records = has_workspace_members;
        }
        permission.can_read_some_records = permission.can_read_all_records;
        object_permissions.insert(object.id.to_string(), permission);
    }
    let mut permissions = RoleRecordsPermissions {
        can_read_all_records: role.can_read_all_object_records,
        can_update_all_records: role.can_update_all_object_records,
        can_soft_delete_all_records: role.can_soft_delete_all_object_records,
        can_destroy_all_records: role.can_destroy_all_object_records,
        objects: object_permissions,
        can_read_some_records: role.can_read_all_object_records,
    };
    restrict_disabled_actions(
        &mut permissions,
        role,
        objects,
        fields,
        acting_application_id,
    );
    permissions.can_read_some_records = permissions
        .objects
        .values()
        .any(ObjectRecordsPermissions::can_read_some_records);
    permissions
}

fn resolve_object_flag(
    object_override: Option<bool>,
    role_flag: bool,
    is_system: bool,
    operation: ObjectRecordsOperationType,
    read_only_system_objects: bool,
) -> bool {
    if let Some(value) = object_override {
        return value;
    }
    match operation {
        ObjectRecordsOperationType::Read => is_system || role_flag,
        ObjectRecordsOperationType::Create
        | ObjectRecordsOperationType::Update
        | ObjectRecordsOperationType::SoftDelete
        | ObjectRecordsOperationType::Destroy => {
            (is_system && !read_only_system_objects) || role_flag
        }
    }
}

pub fn compute_permission_union(permissions: &[&RoleRecordsPermissions]) -> RoleRecordsPermissions {
    let Some(first) = permissions.first() else {
        return RoleRecordsPermissions::empty();
    };
    let mut result = (*first).clone();
    for permission in permissions.iter().skip(1) {
        result.can_read_all_records |= permission.can_read_all_records;
        result.can_update_all_records |= permission.can_update_all_records;
        result.can_soft_delete_all_records |= permission.can_soft_delete_all_records;
        result.can_destroy_all_records |= permission.can_destroy_all_records;
        for (object_id, object) in &permission.objects {
            match result.objects.get_mut(object_id) {
                Some(current) => union_object(current, object),
                None => {
                    result.objects.insert(object_id.clone(), object.clone());
                }
            }
        }
    }
    result.can_read_some_records = result
        .objects
        .values()
        .any(ObjectRecordsPermissions::can_read_some_records);
    result
}

fn union_object(current: &mut ObjectRecordsPermissions, other: &ObjectRecordsPermissions) {
    current.can_create_all_records |= other.can_create_all_records;
    current.can_read_all_records |= other.can_read_all_records;
    current.can_update_all_records |= other.can_update_all_records;
    current.can_soft_delete_all_records |= other.can_soft_delete_all_records;
    current.can_destroy_all_records |= other.can_destroy_all_records;
    current
        .unreadable_fields
        .extend(other.unreadable_fields.iter().cloned());
    current
        .uneditable_fields
        .extend(other.uneditable_fields.iter().cloned());
    dedup_preserving_order(&mut current.unreadable_fields);
    dedup_preserving_order(&mut current.uneditable_fields);
    let rules = current
        .row_level_records_permissions
        .get_or_insert_with(Vec::new)
        .clone();
    current.row_level_records_permissions = Some(union_rules(
        rules,
        other
            .row_level_records_permissions
            .clone()
            .unwrap_or_default(),
    ));
    current.can_read_some_records |= other.can_read_some_records;
}

pub fn compute_permission_intersection(
    permissions: &[&RoleRecordsPermissions],
) -> RoleRecordsPermissions {
    let Some(first) = permissions.first() else {
        return RoleRecordsPermissions::empty();
    };
    let mut result = (*first).clone();
    for permission in permissions.iter().skip(1) {
        result.can_read_all_records &= permission.can_read_all_records;
        result.can_update_all_records &= permission.can_update_all_records;
        result.can_soft_delete_all_records &= permission.can_soft_delete_all_records;
        result.can_destroy_all_records &= permission.can_destroy_all_records;
        result.objects.retain(|object_id, current| {
            let Some(other) = permission.objects.get(object_id) else {
                return false;
            };
            intersect_object(current, other, permission);
            true
        });
    }
    result.can_read_some_records = result
        .objects
        .values()
        .any(ObjectRecordsPermissions::can_read_some_records);
    result
}

fn intersect_object(
    current: &mut ObjectRecordsPermissions,
    other: &ObjectRecordsPermissions,
    other_role: &RoleRecordsPermissions,
) {
    current.can_create_all_records &= other.can_create_all_records;
    current.can_read_all_records &= other.can_read_all_records;
    current.can_update_all_records &= other.can_update_all_records;
    current.can_soft_delete_all_records &= other.can_soft_delete_all_records;
    current.can_destroy_all_records &= other.can_destroy_all_records;
    current
        .unreadable_fields
        .extend(other.unreadable_fields.iter().cloned());
    current
        .uneditable_fields
        .extend(other.uneditable_fields.iter().cloned());
    dedup_preserving_order(&mut current.unreadable_fields);
    dedup_preserving_order(&mut current.uneditable_fields);
    let current_rules = current
        .row_level_records_permissions
        .clone()
        .unwrap_or_default();
    let other_rules = other
        .row_level_records_permissions
        .clone()
        .unwrap_or_default();
    current.row_level_records_permissions =
        Some(intersect_rules(current_rules, other_rules, other_role));
    current.can_read_some_records &= other.can_read_some_records;
}

fn union_rules(
    mut current: Vec<RowLevelRecordsPermissions>,
    other: Vec<RowLevelRecordsPermissions>,
) -> Vec<RowLevelRecordsPermissions> {
    current.extend(other);
    current.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    current.dedup_by(|left, right| left.rule_id == right.rule_id);
    current
}

fn intersect_rules(
    current: Vec<RowLevelRecordsPermissions>,
    other: Vec<RowLevelRecordsPermissions>,
    other_role: &RoleRecordsPermissions,
) -> Vec<RowLevelRecordsPermissions> {
    let other_by_id = other
        .iter()
        .map(|rule| (rule.rule_id.as_str(), rule))
        .collect::<BTreeMap<_, _>>();
    current
        .into_iter()
        .filter_map(|mut rule| {
            let other_rule = other_by_id.get(rule.rule_id.as_str())?;
            rule.can_create_records &=
                other_rule.can_create_records && other_role.can_update_all_records;
            rule.can_read_records &= other_rule.can_read_records && other_role.can_read_all_records;
            rule.can_update_records &=
                other_rule.can_update_records && other_role.can_update_all_records;
            rule.can_soft_delete_records &=
                other_rule.can_soft_delete_records && other_role.can_soft_delete_all_records;
            rule.can_destroy_records &=
                other_rule.can_destroy_records && other_role.can_destroy_all_records;
            rule.unreadable_fields
                .extend(other_rule.unreadable_fields.iter().cloned());
            rule.uneditable_fields
                .extend(other_rule.uneditable_fields.iter().cloned());
            dedup_preserving_order(&mut rule.unreadable_fields);
            dedup_preserving_order(&mut rule.uneditable_fields);
            Some(rule)
        })
        .collect()
}

pub fn compute_context_records_permissions(
    workspace: &WorkspacePermissionSet,
    context: &PermissionsContext,
) -> RoleRecordsPermissions {
    if context.is_system {
        return full_access_records_permissions(workspace.object_ids.iter().copied());
    }
    let mut main_role_ids = context.main_role_ids.clone();
    if main_role_ids.is_empty() {
        main_role_ids = context.role_ids().into_iter().collect();
    }
    let main = compute_permission_union(
        &main_role_ids
            .iter()
            .filter_map(|role_id| workspace.role(*role_id))
            .collect::<Vec<_>>(),
    );
    if context.limiting_role_ids.is_empty() {
        return main;
    }
    let limiting = compute_permission_intersection(
        &context
            .limiting_role_ids
            .iter()
            .filter_map(|role_id| workspace.role(*role_id))
            .collect::<Vec<_>>(),
    );
    compute_permission_intersection(&[&main, &limiting])
}

fn dedup_preserving_order(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn object(id: Uuid, name: &str, is_system: bool) -> FlatObjectMetadata {
        FlatObjectMetadata {
            id,
            name_singular: name.to_string(),
            name_plural: format!("{name}s"),
            is_system,
            ..Default::default()
        }
    }

    #[test]
    fn system_objects_default_to_read_write_for_a_member() {
        let object_id = Uuid::new_v4();
        let role = RoleDefinition::empty(Uuid::new_v4());
        let permissions = compute_role_records_permissions(
            &role,
            &[object(object_id, "custom", true)],
            &[],
            false,
            None,
        );
        let object = &permissions.objects[&object_id.to_string()];
        assert!(object.can_read_all_records);
        assert!(object.can_update_all_records);
        assert!(object.can_create_all_records);
    }

    #[test]
    fn non_system_objects_default_to_denied_for_a_member() {
        let object_id = Uuid::new_v4();
        let role = RoleDefinition::empty(Uuid::new_v4());
        let permissions = compute_role_records_permissions(
            &role,
            &[object(object_id, "person", false)],
            &[],
            false,
            None,
        );
        let object = &permissions.objects[&object_id.to_string()];
        assert!(!object.can_read_all_records);
        assert!(!object.can_update_all_records);
        assert!(!object.can_read_some_records);
    }

    #[test]
    fn object_override_denies_even_an_admin_role() {
        let object_id = Uuid::new_v4();
        let mut role = RoleDefinition::empty(Uuid::new_v4());
        role.can_read_all_object_records = true;
        role.object_permissions.insert(
            object_id,
            super::super::model::ObjectPermissionDefinition {
                object_metadata_id: object_id,
                can_read_object_records: Some(false),
                can_update_object_records: None,
                can_soft_delete_object_records: None,
                can_destroy_object_records: None,
                can_create_object_records: None,
            },
        );
        let permissions = compute_role_records_permissions(
            &role,
            &[object(object_id, "person", false)],
            &[],
            false,
            None,
        );
        assert!(!permissions.objects[&object_id.to_string()].can_read_all_records);
    }

    #[test]
    fn workspace_members_are_readable_but_writes_require_the_flag() {
        let object_id = Uuid::new_v4();
        let role = RoleDefinition::empty(Uuid::new_v4());
        let permissions = compute_role_records_permissions(
            &role,
            &[object(object_id, "workspaceMember", false)],
            &[],
            false,
            None,
        );
        let object = &permissions.objects[&object_id.to_string()];
        assert!(object.can_read_all_records);
        assert!(!object.can_update_all_records);
    }

    #[test]
    fn workflow_runs_follow_the_workflows_flag() {
        let object_id = Uuid::new_v4();
        let mut role = RoleDefinition::empty(Uuid::new_v4());
        role.permission_flags.insert("WORKFLOWS".to_string());
        let permissions = compute_role_records_permissions(
            &role,
            &[object(object_id, "workflowRun", false)],
            &[],
            false,
            None,
        );
        assert!(permissions.objects[&object_id.to_string()].can_read_all_records);
        assert!(permissions.objects[&object_id.to_string()].can_destroy_all_records);
    }

    #[test]
    fn union_grants_when_any_role_grants_and_unions_field_masks() {
        let object_id = Uuid::new_v4();
        let first_field = Uuid::new_v4();
        let second_field = Uuid::new_v4();
        let first = RoleRecordsPermissions {
            can_read_all_records: true,
            objects: [(
                object_id.to_string(),
                ObjectRecordsPermissions {
                    can_read_all_records: true,
                    unreadable_fields: vec![first_field.to_string()],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let second = RoleRecordsPermissions {
            objects: [(
                object_id.to_string(),
                ObjectRecordsPermissions {
                    unreadable_fields: vec![second_field.to_string()],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let union = compute_permission_union(&[&first, &second]);
        assert!(union.can_read_all_records);
        assert_eq!(
            union.objects[&object_id.to_string()]
                .unreadable_fields
                .len(),
            2
        );
    }

    #[test]
    fn intersection_requires_every_limiting_role_to_grant() {
        let object_id = Uuid::new_v4();
        let first = RoleRecordsPermissions {
            can_read_all_records: true,
            can_update_all_records: false,
            can_soft_delete_all_records: false,
            can_destroy_all_records: false,
            objects: [(
                object_id.to_string(),
                ObjectRecordsPermissions {
                    can_read_all_records: true,
                    can_read_some_records: true,
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            can_read_some_records: true,
        };
        let second = RoleRecordsPermissions::empty();
        let intersection = compute_permission_intersection(&[&first, &second]);
        assert!(!intersection.can_read_all_records);
        assert!(intersection.objects.is_empty());
    }

    #[test]
    fn disabled_object_actions_override_role_flags() {
        let object_id = Uuid::new_v4();
        let mut role = RoleDefinition::empty(Uuid::new_v4());
        role.can_read_all_object_records = true;
        let objects = vec![FlatObjectMetadata {
            id: object_id,
            name_singular: "person".to_string(),
            name_plural: "people".to_string(),
            disabled_object_permission_actions: Some(json!(["canReadObjectRecords"])),
            ..Default::default()
        }];
        let permissions = compute_role_records_permissions(&role, &objects, &[], false, None);
        assert!(!permissions.objects[&object_id.to_string()].can_read_all_records);
        assert!(!permissions.objects[&object_id.to_string()].can_read_some_records);
    }
}
