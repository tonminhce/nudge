use std::collections::BTreeMap;
use std::collections::HashSet;

use serde_json::Value;
use uuid::Uuid;

use crate::metadata::flat::FieldMetadataType;
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
        let has_ai_access = object.name_singular != "agentChatThread" || has_ai;
        let row_level_records_permissions = role
            .row_level_permission_rules
            .iter()
            .filter(|rule| rule.object_metadata_id == object.id)
            .filter_map(|rule| {
                let predicates = role
                    .row_level_permission_predicates
                    .iter()
                    .filter(|predicate| predicate.row_level_permission_rule_id == Some(rule.id))
                    .collect::<Vec<_>>();
                if predicates.is_empty() {
                    return None;
                }
                let predicate_groups = role
                    .row_level_permission_predicate_groups
                    .iter()
                    .filter(|group| group.row_level_permission_rule_id == Some(rule.id))
                    .collect::<Vec<_>>();
                let (unreadable_fields, uneditable_fields) =
                    if object.name_singular == "workflowRun" {
                        (Vec::new(), Vec::new())
                    } else {
                        build_rule_field_masks(role, rule.id, fields)
                    };
                Some(RowLevelRecordsPermissions {
                    rule_id: rule.id.to_string(),
                    can_create_records: rule.can_create_records
                        && has_ai_access
                        && object_operation_enabled(
                            object,
                            ObjectRecordsOperationType::Create,
                            acting_application_id,
                        ),
                    can_read_records: rule.can_read_records
                        && has_ai_access
                        && object_operation_enabled(
                            object,
                            ObjectRecordsOperationType::Read,
                            acting_application_id,
                        ),
                    can_update_records: rule.can_update_records
                        && has_ai_access
                        && object_operation_enabled(
                            object,
                            ObjectRecordsOperationType::Update,
                            acting_application_id,
                        ),
                    can_soft_delete_records: rule.can_soft_delete_records
                        && has_ai_access
                        && object_operation_enabled(
                            object,
                            ObjectRecordsOperationType::SoftDelete,
                            acting_application_id,
                        ),
                    can_destroy_records: rule.can_destroy_records
                        && has_ai_access
                        && object_operation_enabled(
                            object,
                            ObjectRecordsOperationType::Destroy,
                            acting_application_id,
                        ),
                    predicates: predicates
                        .into_iter()
                        .map(|predicate| {
                            serde_json::to_value(predicate)
                                .expect("row-level permission predicate serializes")
                        })
                        .collect(),
                    predicate_groups: predicate_groups
                        .into_iter()
                        .map(|group| {
                            serde_json::to_value(group)
                                .expect("row-level permission predicate group serializes")
                        })
                        .collect(),
                    unreadable_fields,
                    uneditable_fields,
                })
            })
            .collect::<Vec<_>>();
        let has_row_level_records_permissions = !row_level_records_permissions.is_empty();
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
            row_level_records_permissions: has_row_level_records_permissions
                .then_some(row_level_records_permissions),
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
        permission.can_read_some_records =
            permission.can_access_some_records(ObjectRecordsOperationType::Read);
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
    refresh_can_read_some_records(&mut permissions);
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

fn object_operation_enabled(
    object: &FlatObjectMetadata,
    operation: ObjectRecordsOperationType,
    acting_application_id: Option<Uuid>,
) -> bool {
    if acting_application_id == Some(object.application_id) {
        return true;
    }
    object
        .disabled_object_permission_actions
        .as_ref()
        .and_then(Value::as_array)
        .is_none_or(|actions| {
            let action = match operation {
                ObjectRecordsOperationType::Create => "canCreateObjectRecords",
                ObjectRecordsOperationType::Read => "canReadObjectRecords",
                ObjectRecordsOperationType::Update => "canUpdateObjectRecords",
                ObjectRecordsOperationType::SoftDelete => "canSoftDeleteObjectRecords",
                ObjectRecordsOperationType::Destroy => "canDestroyObjectRecords",
            };
            !actions
                .iter()
                .any(|disabled| disabled.as_str() == Some(action))
        })
}

fn build_rule_field_masks(
    role: &RoleDefinition,
    rule_id: Uuid,
    fields: &[FlatFieldMetadata],
) -> (Vec<String>, Vec<String>) {
    let mut unreadable_fields = Vec::new();
    let mut uneditable_fields = Vec::new();
    for permission in role
        .field_permissions
        .iter()
        .filter(|permission| permission.row_level_permission_rule_id == Some(rule_id))
    {
        let restrict_read = permission.can_read_field_value == Some(false);
        let restrict_update = permission.can_update_field_value == Some(false);
        for field_id in resolve_rule_field_ids(permission.field_metadata_id, fields) {
            if restrict_read {
                unreadable_fields.push(field_id.to_string());
            }
            if restrict_read || restrict_update {
                uneditable_fields.push(field_id.to_string());
            }
        }
    }
    dedup_preserving_order(&mut unreadable_fields);
    dedup_preserving_order(&mut uneditable_fields);
    (unreadable_fields, uneditable_fields)
}

fn resolve_rule_field_ids(field_id: Uuid, fields: &[FlatFieldMetadata]) -> Vec<Uuid> {
    let Some(field) = fields.iter().find(|field| field.id == field_id) else {
        return vec![field_id];
    };
    if !is_morph_relation_head(field) {
        return vec![field_id];
    }
    let mut field_ids = vec![field_id];
    field_ids.extend(
        fields
            .iter()
            .filter(|candidate| {
                candidate.morph_id == field.morph_id && !is_morph_relation_head(candidate)
            })
            .map(|candidate| candidate.id),
    );
    field_ids
}

fn is_morph_relation_head(field: &FlatFieldMetadata) -> bool {
    field.field_type == FieldMetadataType::MorphRelation
        && field
            .morph_id
            .and_then(|morph_id| {
                field
                    .universal_identifier
                    .parse::<Uuid>()
                    .ok()
                    .map(|id| id == morph_id)
            })
            .unwrap_or(false)
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
    refresh_can_read_some_records(&mut result);
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
    let mut rules = current
        .row_level_records_permissions
        .clone()
        .unwrap_or_default();
    rules.extend(
        other
            .row_level_records_permissions
            .clone()
            .unwrap_or_default(),
    );
    current.row_level_records_permissions = (!rules.is_empty()).then_some(rules);
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
            intersect_object(current, other);
            true
        });
    }
    refresh_can_read_some_records(&mut result);
    result
}

fn intersect_object(current: &mut ObjectRecordsPermissions, other: &ObjectRecordsPermissions) {
    let current_all = object_all_record_access(current);
    let other_all = object_all_record_access(other);
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
    let mut rules = current_rules
        .into_iter()
        .map(|rule| restrict_rule_to_all_record_access(rule, other_all))
        .collect::<Vec<_>>();
    rules.extend(
        other_rules
            .into_iter()
            .map(|rule| restrict_rule_to_all_record_access(rule, current_all)),
    );
    current.row_level_records_permissions = (!rules.is_empty()).then_some(rules);
    current.can_read_some_records =
        current.can_access_some_records(ObjectRecordsOperationType::Read);
}

fn object_all_record_access(object: &ObjectRecordsPermissions) -> [bool; 5] {
    [
        object.can_create_all_records,
        object.can_read_all_records,
        object.can_update_all_records,
        object.can_soft_delete_all_records,
        object.can_destroy_all_records,
    ]
}

fn restrict_rule_to_all_record_access(
    mut rule: RowLevelRecordsPermissions,
    all_record_access: [bool; 5],
) -> RowLevelRecordsPermissions {
    rule.can_create_records &= all_record_access[0];
    rule.can_read_records &= all_record_access[1];
    rule.can_update_records &= all_record_access[2];
    rule.can_soft_delete_records &= all_record_access[3];
    rule.can_destroy_records &= all_record_access[4];
    rule
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
    let limiting_permissions = context
        .limiting_role_ids
        .iter()
        .filter_map(|role_id| workspace.role(*role_id))
        .cloned()
        .map(to_limiting_role_records_permissions)
        .collect::<Vec<_>>();
    let limiting =
        compute_permission_intersection(&limiting_permissions.iter().collect::<Vec<_>>());
    compute_permission_intersection(&[&main, &limiting])
}

fn to_limiting_role_records_permissions(
    mut permissions: RoleRecordsPermissions,
) -> RoleRecordsPermissions {
    for object in permissions.objects.values_mut() {
        object.row_level_records_permissions = None;
    }
    refresh_can_read_some_records(&mut permissions);
    permissions
}

fn refresh_can_read_some_records(permissions: &mut RoleRecordsPermissions) {
    for object in permissions.objects.values_mut() {
        object.can_read_some_records =
            object.can_access_some_records(ObjectRecordsOperationType::Read);
    }
    permissions.can_read_some_records = permissions
        .objects
        .values()
        .any(ObjectRecordsPermissions::can_read_some_records);
}

fn dedup_preserving_order(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

#[cfg(test)]
mod tests {
    use super::super::model::FieldPermissionDefinition;
    use super::super::model::PermissionPrincipalKind;
    use super::super::model::RowLevelPermissionPredicateDefinition;
    use super::super::model::RowLevelPermissionPredicateGroupDefinition;
    use super::super::model::RowLevelPermissionRuleDefinition;
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

    fn rule_definition(
        id: Uuid,
        object_id: Uuid,
        can_read_records: bool,
        can_update_records: bool,
    ) -> RowLevelPermissionRuleDefinition {
        RowLevelPermissionRuleDefinition {
            id,
            object_metadata_id: object_id,
            can_create_records: false,
            can_read_records,
            can_update_records,
            can_soft_delete_records: false,
            can_destroy_records: false,
        }
    }

    fn predicate_definition(
        id: Uuid,
        object_id: Uuid,
        role_id: Uuid,
        rule_id: Uuid,
        group_id: Option<Uuid>,
        position: Option<f64>,
    ) -> RowLevelPermissionPredicateDefinition {
        RowLevelPermissionPredicateDefinition {
            id,
            field_metadata_id: Uuid::new_v4(),
            relation_field_metadata_id: None,
            object_metadata_id: object_id,
            operand: "IS".to_string(),
            sub_field_name: None,
            workspace_member_field_metadata_id: None,
            workspace_member_sub_field_name: None,
            workspace_member_relation_field_metadata_id: None,
            row_level_permission_predicate_group_id: group_id,
            position_in_row_level_permission_predicate_group: position,
            role_id,
            value: Some(json!("active")),
            row_level_permission_rule_id: Some(rule_id),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn predicate_group_definition(
        id: Uuid,
        object_id: Uuid,
        role_id: Uuid,
        rule_id: Uuid,
        parent_id: Option<Uuid>,
        logical_operator: &str,
        position: f64,
        can_update_records: bool,
    ) -> RowLevelPermissionPredicateGroupDefinition {
        RowLevelPermissionPredicateGroupDefinition {
            id,
            parent_row_level_permission_predicate_group_id: parent_id,
            logical_operator: logical_operator.to_string(),
            position_in_row_level_permission_predicate_group: Some(position),
            can_read_records: true,
            can_update_records,
            can_soft_delete_records: false,
            can_destroy_records: false,
            role_id,
            object_metadata_id: object_id,
            row_level_permission_rule_id: Some(rule_id),
        }
    }

    fn role_with_read_rule(role_id: Uuid, object_id: Uuid, rule_id: Uuid) -> RoleDefinition {
        let mut role = RoleDefinition::empty(role_id);
        role.row_level_permission_rules
            .push(rule_definition(rule_id, object_id, true, false));
        role.row_level_permission_predicates
            .push(predicate_definition(
                Uuid::new_v4(),
                object_id,
                role_id,
                rule_id,
                None,
                None,
            ));
        role
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

    #[test]
    fn rule_predicate_groups_preserve_the_upstream_and_or_json_shape() {
        let object_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let rule_id = Uuid::new_v4();
        let root_group_id = Uuid::new_v4();
        let child_group_id = Uuid::new_v4();
        let root_predicate =
            predicate_definition(Uuid::new_v4(), object_id, role_id, rule_id, None, None);
        let child_predicate = predicate_definition(
            Uuid::new_v4(),
            object_id,
            role_id,
            rule_id,
            Some(child_group_id),
            Some(1.0),
        );
        let root_group = predicate_group_definition(
            root_group_id,
            object_id,
            role_id,
            rule_id,
            None,
            "AND",
            0.0,
            true,
        );
        let child_group = predicate_group_definition(
            child_group_id,
            object_id,
            role_id,
            rule_id,
            Some(root_group_id),
            "OR",
            0.0,
            false,
        );
        let mut role = RoleDefinition::empty(role_id);
        role.row_level_permission_rules
            .push(rule_definition(rule_id, object_id, true, true));
        role.row_level_permission_predicates
            .extend([root_predicate.clone(), child_predicate.clone()]);
        role.row_level_permission_predicate_groups
            .extend([root_group.clone(), child_group.clone()]);
        let permissions = compute_role_records_permissions(
            &role,
            &[object(object_id, "person", false)],
            &[],
            false,
            None,
        );
        let object = &permissions.objects[&object_id.to_string()];
        assert!(object.can_read_some_records);
        let actual = serde_json::to_value(
            object
                .row_level_records_permissions
                .as_ref()
                .unwrap()
                .first()
                .unwrap(),
        )
        .unwrap();
        let expected = json!({
            "ruleId": rule_id,
            "canCreateRecords": false,
            "canReadRecords": true,
            "canUpdateRecords": true,
            "canSoftDeleteRecords": false,
            "canDestroyRecords": false,
            "predicates": [
                {
                    "id": root_predicate.id,
                    "fieldMetadataId": root_predicate.field_metadata_id,
                    "relationFieldMetadataId": null,
                    "objectMetadataId": object_id,
                    "operand": "IS",
                    "subFieldName": null,
                    "workspaceMemberFieldMetadataId": null,
                    "workspaceMemberSubFieldName": null,
                    "workspaceMemberRelationFieldMetadataId": null,
                    "rowLevelPermissionPredicateGroupId": null,
                    "positionInRowLevelPermissionPredicateGroup": null,
                    "roleId": role_id,
                    "value": "active",
                    "rowLevelPermissionRuleId": rule_id
                },
                {
                    "id": child_predicate.id,
                    "fieldMetadataId": child_predicate.field_metadata_id,
                    "relationFieldMetadataId": null,
                    "objectMetadataId": object_id,
                    "operand": "IS",
                    "subFieldName": null,
                    "workspaceMemberFieldMetadataId": null,
                    "workspaceMemberSubFieldName": null,
                    "workspaceMemberRelationFieldMetadataId": null,
                    "rowLevelPermissionPredicateGroupId": child_group_id,
                    "positionInRowLevelPermissionPredicateGroup": 1.0,
                    "roleId": role_id,
                    "value": "active",
                    "rowLevelPermissionRuleId": rule_id
                }
            ],
            "predicateGroups": [
                {
                    "id": root_group.id,
                    "parentRowLevelPermissionPredicateGroupId": null,
                    "logicalOperator": "AND",
                    "positionInRowLevelPermissionPredicateGroup": 0.0,
                    "canReadRecords": true,
                    "canUpdateRecords": true,
                    "canSoftDeleteRecords": false,
                    "canDestroyRecords": false,
                    "roleId": role_id,
                    "objectMetadataId": object_id,
                    "rowLevelPermissionRuleId": rule_id
                },
                {
                    "id": child_group.id,
                    "parentRowLevelPermissionPredicateGroupId": root_group_id,
                    "logicalOperator": "OR",
                    "positionInRowLevelPermissionPredicateGroup": 0.0,
                    "canReadRecords": true,
                    "canUpdateRecords": false,
                    "canSoftDeleteRecords": false,
                    "canDestroyRecords": false,
                    "roleId": role_id,
                    "objectMetadataId": object_id,
                    "rowLevelPermissionRuleId": rule_id
                }
            ],
            "unreadableFields": [],
            "uneditableFields": []
        });
        assert_eq!(actual, expected);
    }

    #[test]
    fn main_role_union_concatenates_rules_without_reordering() {
        let object_id = Uuid::new_v4();
        let first_rule_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let second_rule_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let first_role = role_with_read_rule(Uuid::new_v4(), object_id, first_rule_id);
        let second_role = role_with_read_rule(Uuid::new_v4(), object_id, second_rule_id);
        let first = compute_role_records_permissions(
            &first_role,
            &[object(object_id, "person", false)],
            &[],
            false,
            None,
        );
        let second = compute_role_records_permissions(
            &second_role,
            &[object(object_id, "person", false)],
            &[],
            false,
            None,
        );
        let union = compute_permission_union(&[&first, &second]);
        let object = &union.objects[&object_id.to_string()];
        let rule_ids = object
            .row_level_records_permissions
            .as_ref()
            .unwrap()
            .iter()
            .map(|rule| rule.rule_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            rule_ids,
            vec![first_rule_id.to_string(), second_rule_id.to_string()]
        );
        assert!(object.can_read_some_records);
        assert!(union.can_read_some_records);
    }

    #[test]
    fn limiting_roles_discard_rules_and_gate_main_rules_by_object_access() {
        let object_id = Uuid::new_v4();
        let main_role_id = Uuid::new_v4();
        let limiting_role_id = Uuid::new_v4();
        let main_rule_id = Uuid::new_v4();
        let limiting_rule_id = Uuid::new_v4();
        let rule_field_id = Uuid::new_v4();
        let mut main_definition = role_with_read_rule(main_role_id, object_id, main_rule_id);
        main_definition.row_level_permission_rules[0].can_update_records = true;
        main_definition
            .field_permissions
            .push(FieldPermissionDefinition {
                object_metadata_id: object_id,
                field_metadata_id: rule_field_id,
                can_read_field_value: Some(false),
                can_update_field_value: None,
                row_level_permission_rule_id: Some(main_rule_id),
            });
        let main = compute_role_records_permissions(
            &main_definition,
            &[object(object_id, "person", false)],
            &[],
            false,
            None,
        );
        let mut limiting_definition = RoleDefinition::empty(limiting_role_id);
        limiting_definition.can_read_all_object_records = false;
        limiting_definition.can_update_all_object_records = true;
        limiting_definition.object_permissions.insert(
            object_id,
            super::super::model::ObjectPermissionDefinition {
                object_metadata_id: object_id,
                can_read_object_records: Some(true),
                can_update_object_records: Some(false),
                can_soft_delete_object_records: None,
                can_destroy_object_records: None,
                can_create_object_records: None,
            },
        );
        limiting_definition
            .row_level_permission_rules
            .push(rule_definition(limiting_rule_id, object_id, true, false));
        limiting_definition
            .row_level_permission_predicates
            .push(predicate_definition(
                Uuid::new_v4(),
                object_id,
                limiting_role_id,
                limiting_rule_id,
                None,
                None,
            ));
        let limiting = compute_role_records_permissions(
            &limiting_definition,
            &[object(object_id, "person", false)],
            &[],
            false,
            None,
        );
        let workspace = WorkspacePermissionSet {
            roles: [(main_role_id, main), (limiting_role_id, limiting)]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let context = PermissionsContext {
            workspace_id: None,
            principal_kind: PermissionPrincipalKind::WorkspaceMember,
            user_id: None,
            user_workspace_id: None,
            workspace_member_id: None,
            api_key_id: None,
            agent_id: None,
            application_id: None,
            main_role_ids: vec![main_role_id],
            limiting_role_ids: vec![limiting_role_id],
            is_system: false,
        };
        let permissions = compute_context_records_permissions(&workspace, &context);
        let object = &permissions.objects[&object_id.to_string()];
        let rules = object.row_level_records_permissions.as_ref().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].rule_id, main_rule_id.to_string());
        assert!(rules[0].can_read_records);
        assert!(!rules[0].can_update_records);
        assert_eq!(rules[0].unreadable_fields, vec![rule_field_id.to_string()]);
        assert!(!object.can_read_all_records);
        assert!(!object.can_update_all_records);
        assert!(object.can_read_some_records);
        assert!(permissions.can_read_some_records);
    }

    #[test]
    fn rules_without_predicates_are_omitted() {
        let object_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let rule_id = Uuid::new_v4();
        let mut role = RoleDefinition::empty(role_id);
        role.row_level_permission_rules
            .push(rule_definition(rule_id, object_id, true, true));
        role.row_level_permission_predicate_groups
            .push(predicate_group_definition(
                Uuid::new_v4(),
                object_id,
                role_id,
                rule_id,
                None,
                "AND",
                0.0,
                true,
            ));
        let permissions = compute_role_records_permissions(
            &role,
            &[object(object_id, "person", false)],
            &[],
            false,
            None,
        );
        let object = &permissions.objects[&object_id.to_string()];
        assert!(object.row_level_records_permissions.is_none());
        assert!(!object.can_read_some_records);
        assert!(!permissions.can_read_some_records);
    }

    #[test]
    fn rule_field_permissions_create_rule_specific_masks() {
        let object_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();
        let rule_id = Uuid::new_v4();
        let role_field_id = Uuid::new_v4();
        let label_field_id = Uuid::new_v4();
        let update_only_field_id = Uuid::new_v4();
        let mut role = role_with_read_rule(role_id, object_id, rule_id);
        role.field_permissions.extend([
            FieldPermissionDefinition {
                object_metadata_id: object_id,
                field_metadata_id: role_field_id,
                can_read_field_value: Some(false),
                can_update_field_value: None,
                row_level_permission_rule_id: None,
            },
            FieldPermissionDefinition {
                object_metadata_id: object_id,
                field_metadata_id: label_field_id,
                can_read_field_value: Some(false),
                can_update_field_value: None,
                row_level_permission_rule_id: Some(rule_id),
            },
            FieldPermissionDefinition {
                object_metadata_id: object_id,
                field_metadata_id: update_only_field_id,
                can_read_field_value: None,
                can_update_field_value: Some(false),
                row_level_permission_rule_id: Some(rule_id),
            },
        ]);
        let object_metadata = FlatObjectMetadata {
            label_identifier_field_metadata_id: Some(label_field_id),
            ..object(object_id, "person", false)
        };
        let permissions =
            compute_role_records_permissions(&role, &[object_metadata], &[], false, None);
        let object = &permissions.objects[&object_id.to_string()];
        assert_eq!(object.unreadable_fields, vec![role_field_id.to_string()]);
        assert_eq!(object.uneditable_fields, vec![role_field_id.to_string()]);
        let rule = object
            .row_level_records_permissions
            .as_ref()
            .unwrap()
            .first()
            .unwrap();
        assert_eq!(rule.unreadable_fields, vec![label_field_id.to_string()]);
        assert_eq!(
            rule.uneditable_fields,
            vec![label_field_id.to_string(), update_only_field_id.to_string()]
        );
    }
}
