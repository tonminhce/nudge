use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use crate::metadata::flat::RelationType;
use crate::metadata::names::compute_composite_column_name;
use crate::query::repository::quote_identifier;

use super::actors::ActorContext;
use super::base::execute_json_rows;
use super::base::ObjectModel;
use super::base::RunnerSchema;
use super::base::UniqueField;
use super::create::create_many;
use super::create::CreateArgs;
use super::RunnerError;

const CONNECT: &str = "connect";
const CONNECT_WHERE: &str = "where";
const CREATE: &str = "create";
const DISCONNECT: &str = "disconnect";

#[derive(Debug, Clone, PartialEq)]
struct ConnectCondition {
    column: String,
    value: Value,
}

#[derive(Debug, PartialEq)]
pub enum RelationWriteOperation {
    Passthrough,
    Connect(Map<String, Value>),
    Create(Map<String, Value>),
    Disconnect,
}

pub fn resolve_relation_nested_queries<'a>(
    pool: &'a sqlx::PgPool,
    schema: &'a RunnerSchema,
    object_name_singular: &'a str,
    records: &'a mut [Map<String, Value>],
    actor: Option<&'a ActorContext>,
) -> Pin<Box<dyn Future<Output = Result<(), RunnerError>> + Send + 'a>> {
    Box::pin(async move {
        let model = schema.object(object_name_singular)?;

        for record in records.iter_mut() {
            resolve_record_relation_queries(pool, schema, model, record, actor).await?;
        }

        Ok(())
    })
}

async fn resolve_record_relation_queries(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    model: &ObjectModel,
    record: &mut Map<String, Value>,
    actor: Option<&ActorContext>,
) -> Result<(), RunnerError> {
    let keys = record.keys().cloned().collect::<Vec<_>>();

    for key in keys {
        let Some(relation) = model.shape.relation(&key) else {
            continue;
        };

        if relation.relation_type != RelationType::ManyToOne {
            continue;
        }

        let Some(join_column) = relation.join_column_name.clone() else {
            continue;
        };
        let Some(value) = record.get(&key).cloned() else {
            continue;
        };
        let target = relation
            .target_object_metadata_id
            .and_then(|id| schema.object_by_id(&id).ok());

        match plan_relation_write(&key, &value)? {
            RelationWriteOperation::Passthrough => {}
            RelationWriteOperation::Disconnect => {
                reject_conflicting_join_column(record, &key, &join_column)?;
                record.remove(&key);
                record.insert(join_column, Value::Null);
            }
            RelationWriteOperation::Create(nested) => {
                if record.contains_key(&join_column) {
                    return Err(RunnerError::new(
                        "INVALID_ARGS_DATA",
                        format!("Cannot provide both {key}.create and {join_column}"),
                    ));
                }

                let target = target.ok_or_else(|| malformed_target(&key))?;
                let created = create_many(
                    pool,
                    schema,
                    &target.object.name_singular,
                    &CreateArgs {
                        records: vec![nested],
                        actor: actor.cloned(),
                        ..Default::default()
                    },
                )
                .await?;
                let connected_id = created.first().map(Uuid::to_string).unwrap_or_default();

                if connected_id.is_empty() {
                    return Err(RunnerError::new(
                        "INVALID_ARGS_DATA",
                        format!(
                            "Expected 1 created {} record, received 0",
                            target.object.name_singular
                        ),
                    ));
                }

                record.remove(&key);
                record.insert(join_column, Value::String(connected_id));
            }
            RelationWriteOperation::Connect(where_object) => {
                reject_conflicting_join_column(record, &key, &join_column)?;

                let target = target.ok_or_else(|| malformed_target(&key))?;
                validate_connect_unique_constraint(&key, &where_object, &unique_groups(target))?;
                let Some(conditions) =
                    connect_conditions(&where_object, &composite_field_names(target))
                else {
                    continue;
                };
                let found = fetch_connect_candidates(pool, target, &conditions).await?;

                if found.len() != 1 {
                    return Err(RunnerError::new(
                        "INVALID_ARGS_DATA",
                        format!(
                            "Expected 1 record to connect to {key}, but found {} for {}",
                            found.len(),
                            format_conditions(&conditions)
                        ),
                    ));
                }

                let connected_id = found
                    .first()
                    .and_then(|row| row.get("id"))
                    .and_then(Value::as_str)
                    .filter(|id| Uuid::parse_str(id).is_ok())
                    .ok_or_else(|| {
                        RunnerError::new(
                            "INVALID_ARGS_DATA",
                            format!(
                                "Expected 1 record to connect to {key}, but found 0 for {}",
                                format_conditions(&conditions)
                            ),
                        )
                    })?
                    .to_string();

                record.remove(&key);
                record.insert(join_column, Value::String(connected_id));
            }
        }
    }

    Ok(())
}

fn malformed_target(key: &str) -> RunnerError {
    RunnerError::new(
        "MALFORMED_METADATA",
        format!("Target object metadata not found for {key}"),
    )
}

pub fn plan_relation_write(
    key: &str,
    value: &Value,
) -> Result<RelationWriteOperation, RunnerError> {
    let Some(object) = value.as_object() else {
        return Err(relation_operation_required(key));
    };

    let mut operation = RelationWriteOperation::Passthrough;
    let mut recognised = false;

    if let Some(disconnect) = object.get(DISCONNECT).filter(|value| !value.is_null()) {
        recognised = true;

        if disconnect.as_bool().unwrap_or(false) {
            operation = RelationWriteOperation::Disconnect;
        }
    }

    if let Some(create) = object
        .get(CREATE)
        .and_then(Value::as_object)
        .filter(|create| !create.is_empty())
    {
        recognised = true;
        operation = RelationWriteOperation::Create(create.clone());
    }

    if object.get(CONNECT).is_some_and(|value| !value.is_null()) {
        recognised = true;

        if let Some(where_object) = recognized_connect_where(object.get(CONNECT)) {
            operation = RelationWriteOperation::Connect(where_object);
        }
    }

    if !recognised {
        return Err(relation_operation_required(key));
    }

    Ok(operation)
}

fn relation_operation_required(key: &str) -> RunnerError {
    RunnerError::new(
        "INVALID_ARGS_DATA",
        format!("Relation \"{key}\" requires create, connect, or disconnect operation"),
    )
}

fn recognized_connect_where(connect: Option<&Value>) -> Option<Map<String, Value>> {
    let where_object = connect?
        .as_object()?
        .get(CONNECT_WHERE)?
        .as_object()?
        .clone();

    if where_object.is_empty() {
        return None;
    }

    let scalar_only = where_object.values().all(|value| {
        value.is_string()
            || value
                .as_object()
                .is_some_and(|nested| nested.values().all(Value::is_string))
    });

    if !scalar_only {
        return None;
    }

    Some(where_object)
}

fn connect_conditions(
    where_object: &Map<String, Value>,
    composite_fields: &HashSet<String>,
) -> Option<Vec<ConnectCondition>> {
    let mut conditions = Vec::new();

    for (field_name, value) in where_object {
        match value {
            Value::Object(nested) if composite_fields.contains(field_name) => {
                for (sub_field_name, sub_value) in nested {
                    if !sub_value.is_string() {
                        return None;
                    }

                    conditions.push(ConnectCondition {
                        column: compute_composite_column_name(field_name, sub_field_name),
                        value: sub_value.clone(),
                    });
                }
            }
            Value::Object(_) => return None,
            scalar => conditions.push(ConnectCondition {
                column: field_name.clone(),
                value: scalar.clone(),
            }),
        }
    }

    (!conditions.is_empty()).then_some(conditions)
}

fn composite_field_names(model: &ObjectModel) -> HashSet<String> {
    model
        .shape
        .columns
        .iter()
        .filter_map(|column| column.composite_parent_field_name.clone())
        .collect()
}

pub fn validate_connect_unique_constraint(
    key: &str,
    where_object: &Map<String, Value>,
    groups: &[Vec<UniqueField>],
) -> Result<(), RunnerError> {
    let populated = groups.iter().any(|group| {
        !group.is_empty()
            && group
                .iter()
                .all(|field| where_object.contains_key(&field.field_name))
    });

    if !populated {
        return Err(RunnerError::new(
            "INVALID_ARGS_DATA",
            format!(
                "Missing required fields: at least one unique constraint have to be fully populated for '{key}'."
            ),
        ));
    }

    for field_name in where_object.keys() {
        let known = groups
            .iter()
            .flatten()
            .any(|field| field.field_name == *field_name);

        if !known {
            return Err(RunnerError::new(
                "INVALID_ARGS_DATA",
                format!("Field {field_name} is not a unique constraint field for '{key}'."),
            ));
        }
    }

    Ok(())
}

fn unique_groups(model: &ObjectModel) -> Vec<Vec<UniqueField>> {
    let mut groups = vec![vec![UniqueField {
        field_name: String::from("id"),
        sub_field_name: None,
    }]];

    for group in &model.unique_field_groups {
        if !groups.iter().any(|existing| existing == group) {
            groups.push(group.clone());
        }
    }

    groups
}

fn reject_conflicting_join_column(
    record: &Map<String, Value>,
    key: &str,
    join_column: &str,
) -> Result<(), RunnerError> {
    if record
        .get(join_column)
        .is_some_and(|value| !value.is_null())
    {
        return Err(RunnerError::new(
            "INVALID_ARGS_DATA",
            format!("{key} and {join_column} cannot be both provided."),
        ));
    }

    Ok(())
}

fn format_conditions(conditions: &[ConnectCondition]) -> String {
    conditions
        .iter()
        .map(|condition| {
            let value = match &condition.value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };

            format!("{} = {}", condition.column, value)
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

async fn fetch_connect_candidates(
    pool: &sqlx::PgPool,
    target: &ObjectModel,
    conditions: &[ConnectCondition],
) -> Result<Vec<Map<String, Value>>, RunnerError> {
    let quoted_alias = quote_identifier(&target.object.name_singular)?;
    let mut predicates = Vec::new();
    let mut parameters = Vec::new();

    for condition in conditions {
        predicates.push(format!(
            "{}.{} = ${}",
            quoted_alias,
            quote_identifier(&condition.column)?,
            predicates.len() + 1
        ));

        parameters.push(condition.value.clone());
    }

    if target.shape.has_deleted_at_column {
        predicates.push(format!(
            "{}.{} IS NULL",
            quoted_alias,
            quote_identifier("deletedAt")?
        ));
    }

    let sql = format!(
        "SELECT {}.{} AS {} FROM {}.{} AS {} WHERE {} LIMIT 2",
        quoted_alias,
        quote_identifier("id")?,
        quote_identifier("id")?,
        quote_identifier(&target.shape.schema_name)?,
        quote_identifier(&target.shape.table_name)?,
        quoted_alias,
        predicates.join(" AND ")
    );

    execute_json_rows(pool, &sql, &parameters).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn json_to_map(value: Value) -> Map<String, Value> {
        value.as_object().expect("object value").clone()
    }

    fn composite_fields(field_names: &[&str]) -> HashSet<String> {
        field_names
            .iter()
            .map(|field_name| (*field_name).to_string())
            .collect()
    }

    fn unique_groups_for(field_names: &[&str]) -> Vec<Vec<UniqueField>> {
        let mut groups = vec![vec![UniqueField {
            field_name: String::from("id"),
            sub_field_name: None,
        }]];

        for field_name in field_names {
            if *field_name == "id" {
                continue;
            }

            groups.push(vec![UniqueField {
                field_name: (*field_name).to_string(),
                sub_field_name: None,
            }]);
        }

        groups
    }

    #[test]
    fn a_scalar_relation_value_reports_the_missing_nested_operation() {
        let error = plan_relation_write("company", &json!("8f9a1c8e-1b2c-4d3e-9f0a-1b2c3d4e5f60"))
            .expect_err("scalar rejected");

        assert_eq!(error.code(), "INVALID_ARGS_DATA");
        assert_eq!(
            error.message(),
            "Relation \"company\" requires create, connect, or disconnect operation"
        );
    }

    #[test]
    fn a_null_relation_value_reports_the_missing_nested_operation() {
        let error = plan_relation_write("company", &Value::Null).expect_err("null rejected");

        assert_eq!(
            error.message(),
            "Relation \"company\" requires create, connect, or disconnect operation"
        );
    }

    #[test]
    fn an_object_without_a_nested_operation_reports_the_missing_operation() {
        let error = plan_relation_write("company", &json!({ "name": "Initech" }))
            .expect_err("no operation key");

        assert_eq!(error.code(), "INVALID_ARGS_DATA");
    }

    #[test]
    fn a_connect_without_where_is_left_for_the_column_expansion() {
        let plan =
            plan_relation_write("company", &json!({ "connect": { "id": "a" } })).expect("planned");

        assert_eq!(plan, RelationWriteOperation::Passthrough);
    }

    #[test]
    fn an_empty_connect_where_is_left_for_the_column_expansion() {
        let plan = plan_relation_write("company", &json!({ "connect": { "where": {} } }))
            .expect("planned");

        assert_eq!(plan, RelationWriteOperation::Passthrough);
    }

    #[test]
    fn a_filter_operator_connect_where_yields_no_condition() {
        let plan = plan_relation_write(
            "company",
            &json!({ "connect": { "where": { "name": { "eq": "Initech" } } } }),
        )
        .expect("planned");
        let RelationWriteOperation::Connect(where_object) = plan else {
            panic!("expected a connect plan");
        };

        assert_eq!(
            connect_conditions(&where_object, &composite_fields(&["domainName"])),
            None
        );
    }

    #[test]
    fn a_connect_where_with_an_id_becomes_a_condition_on_the_id_column() {
        let id = "8f9a1c8e-1b2c-4d3e-9f0a-1b2c3d4e5f60";
        let plan = plan_relation_write("company", &json!({ "connect": { "where": { "id": id } } }))
            .expect("planned");
        let RelationWriteOperation::Connect(where_object) = plan else {
            panic!("expected a connect plan");
        };
        let conditions = connect_conditions(&where_object, &composite_fields(&[]))
            .expect("id is a plain column");

        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0].column, "id");
        assert_eq!(format_conditions(&conditions), format!("id = {id}"));
    }

    #[test]
    fn a_composite_connect_where_becomes_a_condition_on_the_flat_column() {
        let plan = plan_relation_write(
            "company",
            &json!({ "connect": { "where": { "domainName": { "primaryLinkUrl": "nope.test" } } } }),
        )
        .expect("planned");
        let RelationWriteOperation::Connect(where_object) = plan else {
            panic!("expected a connect plan");
        };

        assert_eq!(
            format_conditions(
                &connect_conditions(&where_object, &composite_fields(&["domainName"]))
                    .expect("composite expands")
            ),
            "domainNamePrimaryLinkUrl = nope.test"
        );
    }

    #[test]
    fn a_connect_where_matching_no_unique_constraint_is_reported() {
        let groups = unique_groups_for(&["id", "domainName"]);
        let error = validate_connect_unique_constraint(
            "company",
            &json_to_map(json!({ "name": "Initech" })),
            &groups,
        )
        .expect_err("no unique constraint populated");

        assert_eq!(error.code(), "INVALID_ARGS_DATA");
        assert_eq!(
            error.message(),
            "Missing required fields: at least one unique constraint have to be fully populated for 'company'."
        );
    }

    #[test]
    fn a_connect_where_outside_the_unique_constraints_is_reported() {
        let groups = unique_groups_for(&["id"]);
        let error = validate_connect_unique_constraint(
            "company",
            &json_to_map(json!({ "id": "a", "workspaceId": "b" })),
            &groups,
        )
        .expect_err("unknown constraint field");

        assert_eq!(
            error.message(),
            "Field workspaceId is not a unique constraint field for 'company'."
        );
    }

    #[test]
    fn a_disconnect_true_becomes_a_disconnect() {
        let plan = plan_relation_write("company", &json!({ "disconnect": true })).expect("planned");

        assert_eq!(plan, RelationWriteOperation::Disconnect);
    }

    #[test]
    fn a_create_object_becomes_a_create() {
        let plan = plan_relation_write("company", &json!({ "create": { "name": "Initech" } }))
            .expect("planned");
        let RelationWriteOperation::Create(record) = plan else {
            panic!("expected a create plan");
        };

        assert_eq!(record.get("name"), Some(&json!("Initech")));
    }
}
