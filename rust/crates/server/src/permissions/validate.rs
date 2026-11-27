#![allow(dead_code)]

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use uuid::Uuid;

use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::FlatObjectMetadata;

use super::error::PermissionError;
use super::model::ObjectRecordsOperationType;
use super::model::RoleRecordsPermissions;

pub fn assert_operation_allowed(
    permissions: &RoleRecordsPermissions,
    object_metadata_id: Uuid,
    operation: ObjectRecordsOperationType,
) -> Result<(), PermissionError> {
    let object = permissions
        .object(object_metadata_id)
        .ok_or_else(permission_denied)?;
    if object.can_access_some_records(operation) {
        Ok(())
    } else {
        Err(permission_denied())
    }
}

pub fn validate_operation_is_permitted_or_throw(
    permissions: &RoleRecordsPermissions,
    object: &FlatObjectMetadata,
    fields: &[FlatFieldMetadata],
    operation: ObjectRecordsOperationType,
    selected_field_ids: &BTreeSet<Uuid>,
    updated_field_ids: &BTreeSet<Uuid>,
    exempt_updated_field_ids: &BTreeSet<Uuid>,
) -> Result<(), PermissionError> {
    let object_permissions = permissions
        .object(object.id)
        .ok_or_else(|| PermissionError::denied("Object is not accessible"))?;
    let fields_by_id = fields
        .iter()
        .map(|field| (field.id, field))
        .collect::<BTreeMap<_, _>>();
    let unreadable = object_permissions
        .unreadable_fields
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect::<BTreeSet<_>>();
    let uneditable = object_permissions
        .uneditable_fields
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect::<BTreeSet<_>>();

    for field_id in selected_field_ids {
        if unreadable.contains(field_id) {
            return Err(unreadable_field_error(object, &fields_by_id, *field_id));
        }
    }

    if matches!(
        operation,
        ObjectRecordsOperationType::Create | ObjectRecordsOperationType::Update
    ) {
        for field_id in updated_field_ids {
            if exempt_updated_field_ids.contains(field_id) {
                continue;
            }
            if uneditable.contains(field_id) {
                return Err(uneditable_field_error(object, &fields_by_id, *field_id));
            }
        }
    }
    Ok(())
}

fn permission_denied() -> PermissionError {
    PermissionError::denied("Entity performing the request does not have permission")
}

fn unreadable_field_error(
    object: &FlatObjectMetadata,
    fields: &BTreeMap<Uuid, &FlatFieldMetadata>,
    field_id: Uuid,
) -> PermissionError {
    let field_name = fields
        .get(&field_id)
        .map(|field| field.name.as_str())
        .unwrap_or("unknown");
    PermissionError::denied(format!(
        "no permission to read field \"{field_name}\" on \"{}\"",
        object.name_singular
    ))
}

fn uneditable_field_error(
    object: &FlatObjectMetadata,
    fields: &BTreeMap<Uuid, &FlatFieldMetadata>,
    field_id: Uuid,
) -> PermissionError {
    let field_name = fields
        .get(&field_id)
        .map(|field| field.name.as_str())
        .unwrap_or("unknown");
    PermissionError::denied(format!(
        "no permission to write field \"{field_name}\" on \"{}\"",
        object.name_singular
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (
        FlatObjectMetadata,
        FlatFieldMetadata,
        RoleRecordsPermissions,
    ) {
        let object = FlatObjectMetadata {
            id: Uuid::new_v4(),
            name_singular: "person".to_string(),
            name_plural: "people".to_string(),
            ..Default::default()
        };
        let field = FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object.id,
            name: "jobTitle".to_string(),
            ..Default::default()
        };
        let permissions = RoleRecordsPermissions {
            objects: [(
                object.id.to_string(),
                super::super::model::ObjectRecordsPermissions {
                    can_read_some_records: true,
                    unreadable_fields: vec![field.id.to_string()],
                    uneditable_fields: vec![field.id.to_string()],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        (object, field, permissions)
    }

    #[test]
    fn selecting_an_unreadable_field_is_denied() {
        let (object, field, permissions) = fixture();
        let error = validate_operation_is_permitted_or_throw(
            &permissions,
            &object,
            std::slice::from_ref(&field),
            ObjectRecordsOperationType::Read,
            &[field.id].into_iter().collect(),
            &BTreeSet::new(),
            &BTreeSet::new(),
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            super::super::error::PermissionErrorCode::PermissionDenied
        );
        assert!(error.message.contains("jobTitle"));
    }

    #[test]
    fn create_can_write_an_exempt_predicate_field() {
        let (object, field, permissions) = fixture();
        let result = validate_operation_is_permitted_or_throw(
            &permissions,
            &object,
            std::slice::from_ref(&field),
            ObjectRecordsOperationType::Create,
            &BTreeSet::new(),
            &[field.id].into_iter().collect(),
            &[field.id].into_iter().collect(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn update_cannot_write_an_uneditable_field() {
        let (object, field, permissions) = fixture();
        let error = validate_operation_is_permitted_or_throw(
            &permissions,
            &object,
            std::slice::from_ref(&field),
            ObjectRecordsOperationType::Update,
            &BTreeSet::new(),
            &[field.id].into_iter().collect(),
            &BTreeSet::new(),
        )
        .unwrap_err();
        assert!(error.message.contains("write field"));
    }

    #[test]
    fn object_denial_uses_the_upstream_message() {
        let object_id = Uuid::new_v4();
        let permissions = RoleRecordsPermissions {
            objects: [(
                object_id.to_string(),
                super::super::model::ObjectRecordsPermissions::default(),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let error =
            assert_operation_allowed(&permissions, object_id, ObjectRecordsOperationType::Read)
                .unwrap_err();
        assert_eq!(
            error.message,
            "Entity performing the request does not have permission"
        );
    }

    #[test]
    fn create_uses_the_update_permission_gate() {
        let object_id = Uuid::new_v4();
        let permissions = RoleRecordsPermissions {
            objects: [(
                object_id.to_string(),
                super::super::model::ObjectRecordsPermissions {
                    can_update_all_records: true,
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        assert!(assert_operation_allowed(
            &permissions,
            object_id,
            ObjectRecordsOperationType::Create
        )
        .is_ok());
    }
}
