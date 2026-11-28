use std::collections::BTreeSet;

use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use crate::permissions::assert_operation_allowed;
use crate::permissions::validate_operation_is_permitted_or_throw;
use crate::permissions::ObjectRecordsOperationType;
use crate::permissions::PermissionError;
use crate::permissions::RoleRecordsPermissions;
use crate::query::repository::table_shape::ColumnShape;

use super::base::ObjectModel;
use super::base::RunnerSchema;
use super::RunnerError;

pub(crate) fn validate_read(
    schema: &RunnerSchema,
    model: &ObjectModel,
    columns: &[ColumnShape],
) -> Result<(), RunnerError> {
    if schema.denies_company_overlays(&model.object.name_singular) {
        return Err(RunnerError::permission_denied(
            "Entity performing the request does not have permission",
        ));
    }
    let Some(permissions) = schema.records_permissions() else {
        return Ok(());
    };
    assert_operation_allowed(
        permissions,
        model.object.id,
        ObjectRecordsOperationType::Read,
    )
    .map_err(permission_error)?;
    if schema.masks_unreadable_fields() {
        return Ok(());
    }
    let selected = columns
        .iter()
        .map(|column| column.field_metadata_id)
        .collect::<BTreeSet<_>>();
    validate_selected(permissions, model, &selected)
}

pub(crate) fn validate_named_read(
    schema: &RunnerSchema,
    model: &ObjectModel,
    field_names: &[String],
) -> Result<(), RunnerError> {
    if schema.denies_company_overlays(&model.object.name_singular) {
        return Err(RunnerError::permission_denied(
            "Entity performing the request does not have permission",
        ));
    }
    let Some(permissions) = schema.records_permissions() else {
        return Ok(());
    };
    assert_operation_allowed(
        permissions,
        model.object.id,
        ObjectRecordsOperationType::Read,
    )
    .map_err(permission_error)?;
    if schema.masks_unreadable_fields() {
        return Ok(());
    }
    let selected = field_names
        .iter()
        .filter_map(|name| field_id(model, name))
        .collect::<BTreeSet<_>>();
    validate_selected(permissions, model, &selected)
}

pub(crate) fn validate_filter_read(
    schema: &RunnerSchema,
    model: &ObjectModel,
    filter: Option<&Value>,
) -> Result<(), RunnerError> {
    let Some(permissions) = schema.records_permissions() else {
        return Ok(());
    };
    let unreadable = unreadable_fields(permissions, model.object.id);
    validate_filter_value(model, filter, &unreadable)
}

pub(crate) fn validate_order_read(
    schema: &RunnerSchema,
    model: &ObjectModel,
    order_by: Option<&Value>,
) -> Result<(), RunnerError> {
    let Some(permissions) = schema.records_permissions() else {
        return Ok(());
    };
    let unreadable = unreadable_fields(permissions, model.object.id);
    validate_filter_value(model, order_by, &unreadable)
}

pub(crate) fn validate_write(
    schema: &RunnerSchema,
    model: &ObjectModel,
    operation: ObjectRecordsOperationType,
    data: &Map<String, Value>,
    returning_fields: &[String],
) -> Result<(), RunnerError> {
    if schema.denies_company_overlays(&model.object.name_singular) {
        return Err(RunnerError::permission_denied(
            "Entity performing the request does not have permission",
        ));
    }
    let Some(permissions) = schema.records_permissions() else {
        return Ok(());
    };
    assert_operation_allowed(permissions, model.object.id, operation).map_err(permission_error)?;
    let updated = data
        .keys()
        .filter_map(|name| field_id(model, name))
        .collect::<BTreeSet<_>>();
    let selected = if schema.masks_unreadable_fields() {
        BTreeSet::new()
    } else {
        returning_fields
            .iter()
            .filter_map(|name| field_id(model, name))
            .collect::<BTreeSet<_>>()
    };
    let fields = model.fields_by_name.values().cloned().collect::<Vec<_>>();
    validate_operation_is_permitted_or_throw(
        permissions,
        &model.object,
        &fields,
        operation,
        &selected,
        &updated,
        &BTreeSet::new(),
    )
    .map_err(permission_error)
}

fn validate_selected(
    permissions: &RoleRecordsPermissions,
    model: &ObjectModel,
    selected: &BTreeSet<Uuid>,
) -> Result<(), RunnerError> {
    let fields = model.fields_by_name.values().cloned().collect::<Vec<_>>();
    validate_operation_is_permitted_or_throw(
        permissions,
        &model.object,
        &fields,
        ObjectRecordsOperationType::Read,
        selected,
        &BTreeSet::new(),
        &BTreeSet::new(),
    )
    .map_err(permission_error)
}

fn unreadable_fields(
    permissions: &RoleRecordsPermissions,
    object_metadata_id: Uuid,
) -> BTreeSet<Uuid> {
    permissions
        .object(object_metadata_id)
        .map(|object| {
            object
                .unreadable_fields
                .iter()
                .filter_map(|id| Uuid::parse_str(id).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn validate_filter_value(
    model: &ObjectModel,
    value: Option<&Value>,
    unreadable: &BTreeSet<Uuid>,
) -> Result<(), RunnerError> {
    let Some(value) = value else {
        return Ok(());
    };
    match value {
        Value::Object(fields) => {
            for (name, child) in fields {
                if let Some(field_id) = field_id(model, name) {
                    if unreadable.contains(&field_id) {
                        return Err(RunnerError::permission_denied(
                            "Entity performing the request does not have permission",
                        ));
                    }
                }
                validate_filter_value(model, Some(child), unreadable)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                validate_filter_value(model, Some(item), unreadable)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn field_id(model: &ObjectModel, name: &str) -> Option<Uuid> {
    model
        .fields_by_name
        .get(name)
        .map(|field| field.id)
        .or_else(|| {
            model
                .shape
                .column(name)
                .map(|column| column.field_metadata_id)
        })
}

fn permission_error(error: PermissionError) -> RunnerError {
    match error.code.as_str() {
        "RECORD_NOT_EDITABLE" => RunnerError::record_not_editable(),
        "PERMISSION_DENIED" if error.message.starts_with("Entity performing the request") => {
            RunnerError::permission_denied(error.message)
        }
        "PERMISSION_DENIED" => RunnerError::permission_denied(format!(
            "Entity performing the request does not have permission: {}",
            error.message
        )),
        _ => RunnerError::permission_denied(error.message),
    }
}
