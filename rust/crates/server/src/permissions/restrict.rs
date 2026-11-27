use std::collections::BTreeSet;
use std::collections::HashMap;

use uuid::Uuid;

use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::FlatObjectMetadata;

use super::model::ObjectRecordsPermissions;
use super::model::RoleDefinition;
use super::model::RoleRecordsPermissions;

#[derive(Debug, Default, Clone)]
pub(crate) struct FieldMasks {
    unreadable: BTreeSet<Uuid>,
    uneditable: BTreeSet<Uuid>,
}

impl FieldMasks {
    fn restrict_read(&mut self, field_id: Uuid) {
        self.unreadable.insert(field_id);
        self.uneditable.insert(field_id);
    }

    fn restrict_update(&mut self, field_id: Uuid) {
        self.uneditable.insert(field_id);
    }

    fn merge_from(&mut self, other: &FieldMasks) -> bool {
        let unreadable = self.unreadable.len();
        let uneditable = self.uneditable.len();
        self.unreadable.extend(other.unreadable.iter().copied());
        self.uneditable.extend(other.uneditable.iter().copied());
        unreadable != self.unreadable.len() || uneditable != self.uneditable.len()
    }
}

pub fn restrict_disabled_actions(
    permissions: &mut RoleRecordsPermissions,
    role: &RoleDefinition,
    objects: &[FlatObjectMetadata],
    fields: &[FlatFieldMetadata],
    acting_application_id: Option<Uuid>,
) {
    let objects_by_id = objects
        .iter()
        .map(|object| (object.id, object))
        .collect::<HashMap<_, _>>();
    let fields_by_id = fields
        .iter()
        .map(|field| (field.id, field))
        .collect::<HashMap<_, _>>();
    let mut masks = HashMap::<Uuid, FieldMasks>::new();

    for restriction in &role.field_permissions {
        if restriction.row_level_permission_rule_id.is_some() {
            continue;
        }
        let Some(object) = objects_by_id.get(&restriction.object_metadata_id) else {
            continue;
        };
        if acting_application_id == Some(object.application_id) {
            continue;
        }
        let mask = masks.entry(object.id).or_default();
        if restriction.can_read_field_value == Some(false) {
            if object.label_identifier_field_metadata_id != Some(restriction.field_metadata_id) {
                mask.restrict_read(restriction.field_metadata_id);
            } else {
                mask.restrict_update(restriction.field_metadata_id);
            }
        }
        if restriction.can_update_field_value == Some(false) {
            mask.restrict_update(restriction.field_metadata_id);
        }
    }

    for object in objects {
        if acting_application_id == Some(object.application_id) {
            continue;
        }
        let mask = masks.entry(object.id).or_default();
        for field in fields
            .iter()
            .filter(|field| field.object_metadata_id == object.id)
        {
            for action in disabled_actions(&field.disabled_field_permission_actions) {
                match action {
                    "canReadFieldValue" => {
                        if object.label_identifier_field_metadata_id != Some(field.id) {
                            mask.restrict_read(field.id);
                        } else {
                            mask.restrict_update(field.id);
                        }
                    }
                    "canUpdateFieldValue" => mask.restrict_update(field.id),
                    _ => {}
                }
            }
        }
    }

    expand_morph_masks(&mut masks, fields, &fields_by_id);

    for object in objects {
        let owns_object = acting_application_id == Some(object.application_id);
        if let Some(object_permissions) = object_permissions_mut(permissions, object.id) {
            if !owns_object {
                for action in disabled_actions(&object.disabled_object_permission_actions) {
                    match action {
                        "canReadObjectRecords" => {
                            object_permissions.can_read_all_records = false;
                            object_permissions.can_read_some_records = false;
                        }
                        "canUpdateObjectRecords" => {
                            object_permissions.can_update_all_records = false
                        }
                        "canSoftDeleteObjectRecords" => {
                            object_permissions.can_soft_delete_all_records = false
                        }
                        "canDestroyObjectRecords" => {
                            object_permissions.can_destroy_all_records = false
                        }
                        "canCreateObjectRecords" => {
                            object_permissions.can_create_all_records = false
                        }
                        _ => {}
                    }
                }
            }
            if let Some(mask) = masks.get(&object.id) {
                object_permissions.unreadable_fields = ordered_field_ids(object, &mask.unreadable);
                object_permissions.uneditable_fields = ordered_field_ids(object, &mask.uneditable);
            }
        }
    }
}

fn object_permissions_mut(
    permissions: &mut RoleRecordsPermissions,
    object_id: Uuid,
) -> Option<&mut ObjectRecordsPermissions> {
    permissions.objects.get_mut(&object_id.to_string())
}

fn disabled_actions(value: &Option<serde_json::Value>) -> Vec<&str> {
    value
        .as_ref()
        .and_then(serde_json::Value::as_array)
        .map(|actions| {
            actions
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect()
        })
        .unwrap_or_default()
}

fn expand_morph_masks(
    masks: &mut HashMap<Uuid, FieldMasks>,
    fields: &[FlatFieldMetadata],
    fields_by_id: &HashMap<Uuid, &FlatFieldMetadata>,
) {
    loop {
        let snapshot = masks
            .iter()
            .map(|(object_id, mask)| (*object_id, mask.clone()))
            .collect::<HashMap<_, _>>();
        let mut changed = false;
        for (object_id, mask) in &snapshot {
            for field_id in mask.unreadable.iter().chain(mask.uneditable.iter()) {
                let Some(field) = fields_by_id.get(field_id) else {
                    continue;
                };
                if let Some(morph_id) = field.morph_id {
                    for target in fields
                        .iter()
                        .filter(|target| target.morph_id == Some(morph_id))
                    {
                        let target_mask = masks.entry(target.object_metadata_id).or_default();
                        if mask.unreadable.contains(field_id) {
                            target_mask.restrict_read(target.id);
                        } else {
                            target_mask.restrict_update(target.id);
                        }
                        changed |= target_mask.unreadable.len()
                            != snapshot
                                .get(&target.object_metadata_id)
                                .map(|mask| mask.unreadable.len())
                                .unwrap_or(0)
                            || target_mask.uneditable.len()
                                != snapshot
                                    .get(&target.object_metadata_id)
                                    .map(|mask| mask.uneditable.len())
                                    .unwrap_or(0);
                    }
                }
            }
            if masks.entry(*object_id).or_default().merge_from(mask) {
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn ordered_field_ids(object: &FlatObjectMetadata, field_ids: &BTreeSet<Uuid>) -> Vec<String> {
    let order = object
        .field_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), index))
        .collect::<HashMap<_, _>>();
    let mut ordered = field_ids.iter().copied().collect::<Vec<_>>();
    ordered.sort_by_key(|id| order.get(&id.to_string()).copied().unwrap_or(usize::MAX));
    ordered.into_iter().map(|id| id.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn object(id: Uuid, application_id: Uuid, label_id: Option<Uuid>) -> FlatObjectMetadata {
        FlatObjectMetadata {
            id,
            application_id,
            label_identifier_field_metadata_id: label_id,
            ..Default::default()
        }
    }

    fn field(id: Uuid, object_id: Uuid) -> FlatFieldMetadata {
        FlatFieldMetadata {
            id,
            object_metadata_id: object_id,
            ..Default::default()
        }
    }

    #[test]
    fn disabled_read_masks_a_field_but_keeps_the_label_readable() {
        let object_id = Uuid::new_v4();
        let label_id = Uuid::new_v4();
        let secret_id = Uuid::new_v4();
        let objects = vec![object(object_id, Uuid::new_v4(), Some(label_id))];
        let mut fields = vec![field(label_id, object_id), field(secret_id, object_id)];
        fields[0].disabled_field_permission_actions = Some(json!(["canReadFieldValue"]));
        fields[1].disabled_field_permission_actions = Some(json!(["canReadFieldValue"]));
        let role = RoleDefinition::empty(Uuid::new_v4());
        let mut permissions = RoleRecordsPermissions {
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
            ..Default::default()
        };
        restrict_disabled_actions(&mut permissions, &role, &objects, &fields, None);
        let object = &permissions.objects[&object_id.to_string()];
        assert!(!object.unreadable_fields.contains(&label_id.to_string()));
        assert!(object.uneditable_fields.contains(&label_id.to_string()));
        assert!(object.unreadable_fields.contains(&secret_id.to_string()));
        assert!(object.uneditable_fields.contains(&secret_id.to_string()));
    }

    #[test]
    fn application_owner_is_exempt_from_disabled_actions() {
        let application_id = Uuid::new_v4();
        let object_id = Uuid::new_v4();
        let objects = vec![FlatObjectMetadata {
            id: object_id,
            application_id,
            disabled_object_permission_actions: Some(json!(["canReadObjectRecords"])),
            ..Default::default()
        }];
        let role = RoleDefinition::empty(Uuid::new_v4());
        let mut permissions = RoleRecordsPermissions {
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
            ..Default::default()
        };
        restrict_disabled_actions(&mut permissions, &role, &objects, &[], Some(application_id));
        assert!(permissions.objects[&object_id.to_string()].can_read_all_records);
    }

    #[test]
    fn relation_masks_propagate_to_the_inverse_field() {
        let source_object = Uuid::new_v4();
        let target_object = Uuid::new_v4();
        let source_field = field(Uuid::new_v4(), source_object);
        let target_field = field(Uuid::new_v4(), target_object);
        let objects = vec![
            object(source_object, Uuid::new_v4(), None),
            object(target_object, Uuid::new_v4(), None),
        ];
        let fields = vec![FlatFieldMetadata {
            relation_target_field_metadata_id: Some(target_field.id),
            disabled_field_permission_actions: Some(json!(["canUpdateFieldValue"])),
            ..source_field
        }];
        let role = RoleDefinition::empty(Uuid::new_v4());
        let mut permissions = RoleRecordsPermissions {
            objects: [(
                source_object.to_string(),
                ObjectRecordsPermissions::default(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        restrict_disabled_actions(&mut permissions, &role, &objects, &fields, None);
        assert!(permissions.objects[&source_object.to_string()]
            .uneditable_fields
            .contains(&source_field.id.to_string()));
    }
}
