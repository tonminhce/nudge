use serde_json::Map;
use serde_json::Value;
use uuid::Uuid;

use crate::metadata::flat::FieldMetadataType;
use crate::query::compile::compile_named_parameters;
use crate::query::filter::Suffixes;
use crate::query::repository::build_column_result_alias;
use crate::query::repository::build_insert_rows;
use crate::query::repository::build_insert_statement;
use crate::query::repository::quote_identifier;
use crate::query::repository::table_shape::TableShape;
use crate::query::repository::QUERY_MAX_RECORDS;

use super::actors::inject_actor_fields_on_create;
use super::actors::ActorContext;
use super::base::compile_runner_filter;
use super::base::resolve_where_clauses;
use super::base::ObjectModel;
use super::base::RunnerSchema;
use super::position::override_position_on_records;
use super::RunnerError;

#[derive(Debug, Clone, Default)]
pub struct CreateArgs {
    pub records: Vec<Map<String, Value>>,
    pub upsert: bool,
    pub backfill_position_undefined: bool,
    pub actor: Option<ActorContext>,
}

pub async fn create_many(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    args: &CreateArgs,
) -> Result<Vec<Uuid>, RunnerError> {
    if args.records.len() > QUERY_MAX_RECORDS {
        return Err(RunnerError::too_many_records_to_update());
    }

    let model = schema.object(object_name_singular)?;
    let alias = object_name_singular;
    let mut records = args.records.clone();

    for (index, record) in records.iter_mut().enumerate() {
        inject_actor_fields_on_create(model, record, args.actor.as_ref())?;
        record.insert(
            String::from("_positionFirstIndex"),
            Value::Number(index.into()),
        );
        record.insert(
            String::from("_positionLastIndex"),
            Value::Number(index.into()),
        );
    }

    override_position_on_records(
        pool,
        model,
        alias,
        &mut records,
        args.backfill_position_undefined,
    )
    .await?;

    if args.upsert {
        let mut ids = Vec::new();

        for record in records {
            if let Some(existing_id) =
                upsert_existing_record(pool, schema, model, alias, &record, args.actor.as_ref())
                    .await?
            {
                ids.push(existing_id);
            } else {
                ids.extend(insert_records(pool, &model.shape, &[record]).await?);
            }
        }

        return Ok(ids);
    }

    insert_records(pool, &model.shape, &records).await
}

pub async fn create_one(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    record: Map<String, Value>,
    upsert: bool,
    backfill_position_undefined: bool,
    actor: Option<&ActorContext>,
) -> Result<Uuid, RunnerError> {
    let args = CreateArgs {
        records: vec![record],
        upsert,
        backfill_position_undefined,
        actor: actor.cloned(),
    };

    create_many(pool, schema, object_name_singular, &args)
        .await?
        .into_iter()
        .next()
        .ok_or_else(RunnerError::record_not_found)
}

async fn insert_records(
    pool: &sqlx::PgPool,
    shape: &TableShape,
    records: &[Map<String, Value>],
) -> Result<Vec<Uuid>, RunnerError> {
    let rows = build_insert_rows(shape, records).map_err(RunnerError::from_repository)?;
    let sql = build_insert_statement(
        shape,
        &rows.column_names,
        &rows.rows,
        &["id".to_string()],
        false,
    )
    .map_err(RunnerError::from_repository)?;
    let compiled = compile_named_parameters(&sql, &rows.parameters)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let mut compiled_text = compiled.text;

    for (parameter_index, column_name) in rows
        .column_names
        .iter()
        .cycle()
        .take(compiled.values.len())
        .enumerate()
    {
        if matches!(column_name.as_str(), "createdBySource" | "updatedBySource") {
            let enum_name =
                crate::metadata::names::compute_postgres_enum_name(&shape.table_name, column_name);
            let parameter = format!("${}", parameter_index + 1);
            let cast = format!(
                "{parameter}::text::{}.{}",
                crate::query::repository::quote_identifier(&shape.schema_name)?,
                crate::query::repository::quote_identifier(&enum_name)?
            );
            compiled_text = compiled_text.replace(&parameter, &cast);
        } else if column_name.ends_with("WorkspaceMemberId") {
            let parameter = format!("${}", parameter_index + 1);
            compiled_text = compiled_text.replace(&parameter, &format!("{parameter}::uuid"));
        }
    }

    let compiled_statement = crate::query::compile::CompiledStatement {
        text: compiled_text,
        values: compiled.values,
    };
    let mut query = sqlx::query_scalar::<_, Uuid>(&compiled_statement.text);

    for (index, value) in compiled_statement.values.iter().enumerate() {
        let column_name = &rows.column_names[index % rows.column_names.len()];
        query = bind_insert_value(query, shape, column_name, value)?;
    }

    query.fetch_all(pool).await.map_err(RunnerError::database)
}

fn bind_insert_value<'q>(
    query: sqlx::query::QueryScalar<'q, sqlx::Postgres, Uuid, sqlx::postgres::PgArguments>,
    shape: &TableShape,
    column_name: &str,
    value: &'q Value,
) -> Result<
    sqlx::query::QueryScalar<'q, sqlx::Postgres, Uuid, sqlx::postgres::PgArguments>,
    RunnerError,
> {
    let field_type = shape
        .column(column_name)
        .map(|column| column.field_metadata_type)
        .unwrap_or(FieldMetadataType::Text);

    if matches!(
        field_type,
        FieldMetadataType::Uuid | FieldMetadataType::Relation | FieldMetadataType::MorphRelation
    ) || column_name.ends_with("WorkspaceMemberId")
    {
        if let Some(text) = value.as_str() {
            let parsed = Uuid::parse_str(text).map_err(|_| RunnerError::invalid_parameter())?;
            return Ok(query.bind(parsed));
        }
    }

    if matches!(field_type, FieldMetadataType::RawJson) {
        return Ok(query.bind(value));
    }

    match value {
        Value::Null => Ok(query.bind(Option::<String>::None)),
        Value::Bool(value) => Ok(query.bind(*value)),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(query.bind(value))
            } else if let Some(value) = value.as_u64() {
                Ok(query.bind(i64::try_from(value).map_err(|_| RunnerError::invalid_parameter())?))
            } else {
                Ok(query.bind(value.as_f64().unwrap_or_default()))
            }
        }
        Value::String(value) => Ok(query.bind(value.as_str())),
        Value::Array(items) => {
            let items = items
                .iter()
                .map(|item| item.as_str().unwrap_or_default().to_string())
                .collect::<Vec<_>>();
            Ok(query.bind(items))
        }
        Value::Object(_) => Ok(query.bind(value.to_string())),
    }
}

async fn upsert_existing_record(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    model: &ObjectModel,
    alias: &str,
    record: &Map<String, Value>,
    actor: Option<&ActorContext>,
) -> Result<Option<Uuid>, RunnerError> {
    let unique_filters = unique_record_filters(model, record)?;

    if unique_filters.is_empty() {
        return Ok(None);
    }

    let mut filter = Map::new();

    if unique_filters.len() == 1 {
        filter = unique_filters
            .into_iter()
            .next()
            .expect("one unique filter");
    } else {
        filter.insert(
            String::from("or"),
            Value::Array(unique_filters.into_iter().map(Value::Object).collect()),
        );
    }

    let mut suffixes = Suffixes::new();
    let filter_value = Value::Object(filter.clone());
    let compiled_filter = compile_runner_filter(
        schema,
        model.object.name_singular.as_str(),
        Some(&filter_value),
        &mut suffixes,
    )?;
    let Some(compiled_filter) = compiled_filter else {
        return Ok(None);
    };
    let mut parameter_map = serde_json::Map::new();
    parameter_map.extend(compiled_filter.params);
    let where_expression = resolve_where_clauses(
        schema,
        model.object.name_singular.as_str(),
        alias,
        &compiled_filter.clauses,
    )?;
    let mut predicates = vec![where_expression];

    if model.shape.has_deleted_at_column {
        predicates.push(format!(
            "{}.{} IS NULL",
            quote_identifier(alias)?,
            quote_identifier("deletedAt")?
        ));
    }

    let sql = format!(
        "SELECT {}.{} AS {} FROM {}.{} AS {} WHERE {} LIMIT 2",
        quote_identifier(alias)?,
        quote_identifier("id")?,
        quote_identifier(&build_column_result_alias(alias, "id"))?,
        quote_identifier(&model.shape.schema_name)?,
        quote_identifier(&model.shape.table_name)?,
        quote_identifier(alias)?,
        predicates.join(" AND ")
    );
    let compiled_sql = compile_named_parameters(&sql, &parameter_map)
        .map_err(|error| RunnerError::new(error.code(), error.message()))?;
    let mut query = sqlx::query_scalar::<_, Uuid>(&compiled_sql.text);

    for value in &compiled_sql.values {
        query = bind_scalar_value(query, value)?;
    }

    let existing = query.fetch_all(pool).await.map_err(RunnerError::database)?;

    if existing.len() > 1 {
        return Err(RunnerError::new(
            "UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT",
            "Multiple existing records match the upsert criteria",
        ));
    }

    let Some(existing_id) = existing.into_iter().next() else {
        return Ok(None);
    };

    let mut update_data = record.clone();

    if model.shape.has_deleted_at_column {
        update_data.insert(String::from("deletedAt"), Value::Null);
    }

    update_data.remove("createdBy");
    super::mutation::run_mutation(
        pool,
        schema,
        model.object.name_singular.as_str(),
        super::mutation::FilteredMutationKind::Update,
        Some(value_object_for_id(existing_id)),
        Some(Value::Object(update_data)),
        actor,
        false,
        vec![String::from("id")],
    )
    .await?;

    Ok(Some(existing_id))
}

fn bind_scalar_value<'q>(
    query: sqlx::query::QueryScalar<'q, sqlx::Postgres, Uuid, sqlx::postgres::PgArguments>,
    value: &'q Value,
) -> Result<
    sqlx::query::QueryScalar<'q, sqlx::Postgres, Uuid, sqlx::postgres::PgArguments>,
    RunnerError,
> {
    match value {
        Value::Null => Ok(query.bind(Option::<String>::None)),
        Value::Bool(value) => Ok(query.bind(*value)),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(query.bind(value))
            } else if let Some(value) = value.as_u64() {
                Ok(query.bind(i64::try_from(value).map_err(|_| RunnerError::invalid_parameter())?))
            } else {
                Ok(query.bind(value.as_f64().unwrap_or_default()))
            }
        }
        Value::String(value) => Ok(query.bind(value.as_str())),
        Value::Array(items) => Ok(query.bind(
            items
                .iter()
                .map(|item| item.as_str().unwrap_or_default().to_string())
                .collect::<Vec<_>>(),
        )),
        Value::Object(_) => Ok(query.bind(value.to_string())),
    }
}

fn unique_record_filters(
    model: &ObjectModel,
    record: &Map<String, Value>,
) -> Result<Vec<Map<String, Value>>, RunnerError> {
    let mut filters = Vec::new();

    for group in &model.unique_field_groups {
        let mut filter = Map::new();
        let mut complete = true;

        for unique_field in group {
            let value = if let Some(sub_field_name) = &unique_field.sub_field_name {
                record
                    .get(&unique_field.field_name)
                    .and_then(Value::as_object)
                    .and_then(|object| object.get(sub_field_name))
            } else {
                record.get(&unique_field.field_name)
            };

            let Some(value) = value else {
                complete = false;
                break;
            };

            if value.is_null() {
                complete = false;
                break;
            }

            let operator_value = if unique_field.sub_field_name.is_some() {
                let mut sub_object = Map::new();
                let mut operator = Map::new();
                operator.insert(String::from("eq"), value.clone());
                sub_object.insert(
                    unique_field
                        .sub_field_name
                        .clone()
                        .expect("sub field checked"),
                    Value::Object(operator),
                );
                Value::Object(sub_object)
            } else {
                let mut operator = Map::new();
                operator.insert(String::from("eq"), value.clone());
                Value::Object(operator)
            };
            filter.insert(unique_field.field_name.clone(), operator_value);
        }

        if complete {
            filters.push(filter);
        }
    }

    Ok(filters)
}

fn value_object_for_id(id: Uuid) -> Value {
    let mut filter = Map::new();
    let mut operator = Map::new();
    operator.insert(String::from("eq"), Value::String(id.to_string()));
    filter.insert(String::from("id"), Value::Object(operator));
    Value::Object(filter)
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::query::runner::find_many;
    use crate::query::runner::find_one;
    use crate::query::runner::find_one::FindOneArgs;
    use crate::query::runner::run_mutation_one;
    use crate::query::runner::test_support;
    use crate::query::runner::FilteredMutationKind;
    use serde_json::json;

    #[tokio::test]
    async fn create_update_delete_restore_and_upsert_follow_the_upstream_contracts() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };

        let _guard = test_support::DB_TEST_LOCK.lock().await;
        let seeded = test_support::seeded_workspace(&url)
            .await
            .expect("seeded mutation workspace");
        let schema = test_support::runner_schema(&seeded).await;
        let member_id = Uuid::nil().to_string();
        let actor =
            ActorContext::manual(Some(member_id.clone()), Some(String::from("Ada Lovelace")));

        let person_id = create_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            test_support::record(
                r#"{"name": {"firstName": "Ada", "lastName": "Lovelace"}, "jobTitle": "Engineer"}"#,
            ),
            false,
            true,
            Some(&actor),
        )
        .await
        .expect("create person");

        let record = find_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            &FindOneArgs {
                filter: Some(json!({"id": {"eq": person_id.to_string()}})),
                selected_field_names: vec![
                    String::from("id"),
                    String::from("name"),
                    String::from("jobTitle"),
                    String::from("createdBy"),
                    String::from("updatedBy"),
                    String::from("position"),
                ],
                ..Default::default()
            },
            &[],
        )
        .await
        .expect("find created person")
        .expect("created person exists");

        assert_eq!(record["jobTitle"], json!("Engineer"));
        assert_eq!(record["createdBy"]["source"], json!("MANUAL"));
        assert_eq!(record["createdBy"]["name"], json!("Ada Lovelace"));
        assert_eq!(record["createdBy"]["workspaceMemberId"], json!(member_id));
        assert_eq!(record["updatedBy"]["source"], json!("MANUAL"));
        assert!(record["position"].is_number());

        let updated = run_mutation_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            FilteredMutationKind::Update,
            Some(value_object_for_id(person_id)),
            Some(json!({"jobTitle": "Chief Engineer"})),
            Some(&actor),
            false,
            vec![String::from("jobTitle")],
        )
        .await
        .expect("update person");
        assert!(updated.is_object());

        let updated_record = find_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            &FindOneArgs {
                filter: Some(json!({"id": {"eq": person_id.to_string()}})),
                selected_field_names: vec![String::from("jobTitle"), String::from("updatedBy")],
                ..Default::default()
            },
            &[],
        )
        .await
        .expect("find updated person")
        .expect("updated person exists");
        assert_eq!(updated_record["jobTitle"], json!("Chief Engineer"));

        run_mutation_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            FilteredMutationKind::SoftDelete,
            Some(value_object_for_id(person_id)),
            None,
            Some(&actor),
            false,
            vec![String::from("deletedAt")],
        )
        .await
        .expect("soft delete person");

        let deleted = find_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            &FindOneArgs {
                filter: Some(json!({"id": {"eq": person_id.to_string()}})),
                ..Default::default()
            },
            &[],
        )
        .await
        .expect("find soft deleted person excludes trashed rows");
        assert!(deleted.is_none());

        let trashed = find_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            &FindOneArgs {
                filter: Some(json!({"id": {"eq": person_id.to_string()}})),
                with_deleted: true,
                ..Default::default()
            },
            &[],
        )
        .await
        .expect("find trashed person")
        .expect("trashed person exists");
        assert!(trashed["deletedAt"].is_string());

        run_mutation_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            FilteredMutationKind::Restore,
            Some(value_object_for_id(person_id)),
            None,
            Some(&actor),
            true,
            vec![String::from("id")],
        )
        .await
        .expect("restore person");

        let restored = find_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            &FindOneArgs {
                filter: Some(json!({"id": {"eq": person_id.to_string()}})),
                ..Default::default()
            },
            &[],
        )
        .await
        .expect("find restored person")
        .expect("restored person exists");
        assert_eq!(restored["jobTitle"], json!("Chief Engineer"));

        run_mutation_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            FilteredMutationKind::Destroy,
            Some(value_object_for_id(person_id)),
            None,
            Some(&actor),
            true,
            vec![String::from("id")],
        )
        .await
        .expect("destroy person");

        let destroyed = find_one(
            &seeded.state.core_pool,
            &schema,
            "person",
            &FindOneArgs {
                filter: Some(json!({"id": {"eq": person_id.to_string()}})),
                with_deleted: true,
                ..Default::default()
            },
            &[],
        )
        .await
        .expect("find destroyed person");
        assert!(destroyed.is_none());

        let company_data = test_support::record(
            r#"{"name": "Upsert Inc", "domainName": {"primaryLinkUrl": "https://upsert.inc"}}"#,
        );
        let first_id = create_one(
            &seeded.state.core_pool,
            &schema,
            "company",
            company_data.clone(),
            true,
            false,
            Some(&actor),
        )
        .await
        .expect("upsert insert");
        let second_id = create_one(
            &seeded.state.core_pool,
            &schema,
            "company",
            company_data,
            true,
            false,
            Some(&actor),
        )
        .await
        .expect("upsert update");

        assert_eq!(first_id, second_id);

        let companies = find_many::find_many(
            &seeded.state.core_pool,
            &schema,
            "company",
            &find_many::FindManyArgs {
                filter: Some(json!({"name": {"eq": "Upsert Inc"}})),
                ..Default::default()
            },
            true,
            &[],
        )
        .await
        .expect("find upserted company");
        assert_eq!(companies.total_count, Some(1));

        test_support::drop_workspace(&seeded).await;
    }
}
