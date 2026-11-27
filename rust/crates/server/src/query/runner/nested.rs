use std::collections::HashMap;

use serde_json::Value;

use crate::query::compile::compile_named_parameters;
use crate::query::filter::Suffixes;
use crate::query::repository::quote_identifier;

use super::base::column_result_alias;
use super::base::execute_json_rows;
use super::base::render_select_list;
use super::base::resolve_where_clauses;
use super::base::RunnerSchema;
use super::base::Selection;
use super::base::QUERY_MAX_RECORDS_FROM_RELATION;
use super::find_many::NestedRelationSelection;
use super::format_result_rows;
use super::permissions::validate_read;
use super::rls::row_access_filter_with_alias;
use super::RunnerError;

pub async fn load_nested_relations(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    records: &mut [Value],
    nested: &[NestedRelationSelection],
) -> Result<(), RunnerError> {
    for selection in nested {
        let (_, relation, child) =
            schema.relation(object_name_singular, &selection.relation_field_name)?;
        let child_alias = format!("{}_{}", object_name_singular, selection.relation_field_name);
        let child_selection = Selection::from_fields(selection.selected_field_names.clone());
        let columns = schema.select_read_columns(child, &child_selection)?;
        validate_read(schema, child, &columns)?;
        let mapped_columns = columns
            .into_iter()
            .map(|column| {
                let result_alias = column_result_alias(&child_alias, &column.column_name);
                (column, result_alias.clone(), result_alias)
            })
            .collect::<Vec<_>>();
        let mut select_sql = render_select_list(&child_alias, &mapped_columns, &[])?;
        let mut suffixes = Suffixes::new();
        let access = row_access_filter_with_alias(
            pool,
            schema,
            &child.object.name_singular,
            crate::permissions::ObjectRecordsOperationType::Read,
            &child_alias,
            &mut suffixes,
        )
        .await?;
        let access_sql = access
            .as_ref()
            .map(|access| {
                resolve_where_clauses(
                    schema,
                    &child.object.name_singular,
                    &child_alias,
                    &access.clauses,
                )
            })
            .transpose()?;
        let access_params = access.map(|access| access.params).unwrap_or_default();

        if let Some(parent_join_column) = relation.join_column_name.as_deref() {
            let mut parent_ids = Vec::new();

            for record in records.iter() {
                let id = record
                    .get(parent_join_column)
                    .or_else(|| record.get(&selection.relation_field_name))
                    .and_then(Value::as_str);

                if let Some(id) = id {
                    parent_ids.push(Value::String(id.to_string()));
                }
            }

            if parent_ids.is_empty() {
                continue;
            }

            let parameters = serde_json::Map::new();
            let mut map = parameters;
            map.insert(String::from("ids"), Value::Array(parent_ids));
            extend_access_parameters(&mut map, access_params)?;
            let access_sql = access_sql
                .as_ref()
                .map(|access_sql| format!(" AND ({access_sql})"))
                .unwrap_or_default();
            let where_sql = format!(
                "WHERE {}.id = ANY(ARRAY[:...ids]::uuid[]){access_sql}",
                quote_identifier(&child_alias)?
            );
            let sql = format!(
                "SELECT {select_sql} FROM {}.{} AS {} {where_sql} ORDER BY {}.id ASC NULLS LAST",
                quote_identifier(&child.shape.schema_name)?,
                quote_identifier(&child.shape.table_name)?,
                quote_identifier(&child_alias)?,
                quote_identifier(&child_alias)?
            );
            let compiled = compile_named_parameters(&sql, &map)
                .map_err(|error| RunnerError::new(error.code(), error.message()))?;
            let rows = execute_json_rows(pool, &compiled.text, &compiled.values).await?;
            let formatted = format_result_rows(child, &mapped_columns, &rows)?;
            let mut by_id = HashMap::new();

            for record in formatted {
                let id = record.get("id").and_then(Value::as_str).map(str::to_string);

                if let Some(id) = id {
                    by_id.insert(id, record);
                }
            }

            for record in records.iter_mut() {
                let id = record
                    .get(parent_join_column)
                    .or_else(|| record.get(&selection.relation_field_name))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let related = id.and_then(|id| by_id.get(&id).cloned());
                insert_record_field(
                    record,
                    &selection.relation_field_name,
                    related.unwrap_or(Value::Null),
                );
            }

            continue;
        }

        let child_join_column =
            schema.child_join_column(object_name_singular, &selection.relation_field_name)?;
        let mut query_columns = mapped_columns.clone();

        if !query_columns
            .iter()
            .any(|(column, _, _)| column.column_name == child_join_column)
        {
            let column = child
                .shape
                .column(&child_join_column)
                .cloned()
                .ok_or_else(|| {
                    RunnerError::unknown_column(&child_join_column, &child.object.name_singular)
                })?;
            let result_alias = column_result_alias(&child_alias, &child_join_column);
            query_columns.push((column, result_alias.clone(), result_alias));
            select_sql = render_select_list(&child_alias, &query_columns, &[])?;
        }

        let mut parent_ids = Vec::new();

        for record in records.iter() {
            if let Some(id) = record.get("id").and_then(Value::as_str) {
                parent_ids.push(Value::String(id.to_string()));
            }
        }

        if parent_ids.is_empty() {
            continue;
        }

        let mut map = serde_json::Map::new();
        map.insert(String::from("ids"), Value::Array(parent_ids));
        extend_access_parameters(&mut map, access_params)?;
        let access_sql = access_sql
            .as_ref()
            .map(|access_sql| format!(" AND ({access_sql})"))
            .unwrap_or_default();
        let live_predicate = if child.shape.has_deleted_at_column {
            format!(
                "{}.{} IS NULL",
                quote_identifier(&child_alias)?,
                quote_identifier("deletedAt")?
            )
        } else {
            String::from("TRUE")
        };
        let where_sql = format!(
            "WHERE {}.{} = ANY(ARRAY[:...ids]::uuid[]) AND {live_predicate}{access_sql}",
            quote_identifier(&child_alias)?,
            quote_identifier(&child_join_column)?
        );
        let sql = format!(
            "SELECT {select_sql} FROM {}.{} AS {} {where_sql} ORDER BY {}.id ASC NULLS LAST",
            quote_identifier(&child.shape.schema_name)?,
            quote_identifier(&child.shape.table_name)?,
            quote_identifier(&child_alias)?,
            quote_identifier(&child_alias)?
        );
        let per_parent = selection
            .first
            .unwrap_or(QUERY_MAX_RECORDS_FROM_RELATION)
            .min(QUERY_MAX_RECORDS_FROM_RELATION);
        let hard_limit = per_parent.saturating_mul(records.len().max(1));
        let sql = format!("{sql} LIMIT {hard_limit}");
        let compiled = compile_named_parameters(&sql, &map)
            .map_err(|error| RunnerError::new(error.code(), error.message()))?;
        let rows = execute_json_rows(pool, &compiled.text, &compiled.values).await?;
        let formatted = format_result_rows(child, &mapped_columns, &rows)?;
        let mut grouped: HashMap<String, Vec<Value>> = HashMap::new();

        for (raw, record) in rows.into_iter().zip(formatted) {
            let raw_join_alias = column_result_alias(&child_alias, &child_join_column);
            let parent_id = raw
                .get(&raw_join_alias)
                .and_then(Value::as_str)
                .map(str::to_string);

            if let Some(parent_id) = parent_id {
                let bucket = grouped.entry(parent_id).or_default();

                if bucket.len() < per_parent {
                    bucket.push(record);
                }
            }
        }

        for record in records.iter_mut() {
            let id = record
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_default();
            let related = grouped.remove(&id).unwrap_or_default();
            insert_record_field(
                record,
                &selection.relation_field_name,
                Value::Array(related),
            );
        }
    }

    Ok(())
}

fn insert_record_field(record: &mut Value, field_name: &str, value: Value) {
    if let Some(object) = record.as_object_mut() {
        object.insert(field_name.to_string(), value);
    }
}

fn extend_access_parameters(
    map: &mut serde_json::Map<String, Value>,
    params: Vec<(String, Value)>,
) -> Result<(), RunnerError> {
    for (name, value) in params {
        if map.contains_key(&name) {
            return Err(RunnerError::new(
                "RECORD_PERMISSION_QUERY_INVALID",
                "Row-level permission query parameter collision detected",
            ));
        }
        map.insert(name, value);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn nested_access_parameters_reject_collisions_with_parent_ids() {
        let mut params = serde_json::Map::new();
        params.insert(String::from("ids"), Value::Array(Vec::new()));
        let error = extend_access_parameters(
            &mut params,
            vec![(String::from("ids"), json!("unexpected"))],
        )
        .unwrap_err();
        assert_eq!(error.code(), "RECORD_PERMISSION_QUERY_INVALID");
    }

    #[test]
    fn nested_access_parameters_keep_distinct_suffixes() {
        let mut suffixes = Suffixes::new();
        let first = format!("name{}", suffixes.take());
        let second = format!("name{}", suffixes.take());
        let mut params = serde_json::Map::new();
        params.insert(String::from("ids"), Value::Array(Vec::new()));
        extend_access_parameters(
            &mut params,
            vec![(first, json!("Ada")), (second, json!("Grace"))],
        )
        .unwrap();
        assert_eq!(params.len(), 3);
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::metadata::flat::FieldMetadataType;
    use crate::metadata::flat::FlatEntityMaps;
    use crate::metadata::flat::FlatFieldMetadata;
    use crate::metadata::flat::FlatObjectMetadata;
    use crate::query::runner::test_support::DB_TEST_LOCK;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn nested_relations_keep_target_rls_in_both_directions() -> Result<(), RunnerError> {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return Ok(());
        };
        let _guard = DB_TEST_LOCK.lock().await;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .map_err(RunnerError::database)?;
        let workspace_id = Uuid::new_v4();
        let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
        let schema_sql = quote_identifier(&schema_name)?;
        let person_table = format!("{schema_sql}.{}", quote_identifier("person")?);
        let company_table = format!("{schema_sql}.{}", quote_identifier("company")?);
        let person_id = Uuid::new_v4();
        let company_id = Uuid::new_v4();
        let person_name = field(person_id, "name");
        let company_name = field(company_id, "name");
        let person_status = field(person_id, "status");
        let company_status = field(company_id, "status");
        let mut company = relation_field(person_id, "company", company_id);
        let mut people = relation_field(company_id, "people", person_id);
        company.relation_target_field_metadata_id = Some(people.id);
        people.relation_target_field_metadata_id = Some(company.id);
        company.settings = Some(json!({ "relationType": "MANY_TO_ONE" }));
        people.settings = Some(json!({ "relationType": "ONE_TO_MANY" }));
        let person_fields = vec![
            field(person_id, "id"),
            person_name,
            person_status.clone(),
            company,
        ];
        let company_fields = vec![
            field(company_id, "id"),
            company_name,
            company_status.clone(),
            people,
        ];
        let mut fields = FlatEntityMaps::default();

        for item in person_fields.iter().chain(&company_fields) {
            fields.add(
                item.id.to_string(),
                item.clone(),
                item.id.to_string(),
                workspace_id.to_string(),
            );
        }

        let person = object(workspace_id, person_id, "person", &person_fields);
        let company_object = object(workspace_id, company_id, "company", &company_fields);
        let person_rule = crate::permissions::RowLevelRecordsPermissions {
            rule_id: Uuid::new_v4().to_string(),
            can_read_records: true,
            predicates: vec![
                json!({
                    "id": Uuid::new_v4().to_string(),
                    "fieldMetadataId": person_status.id.to_string(),
                    "objectMetadataId": person_id.to_string(),
                    "operand": "IS",
                    "value": "ACTIVE"
                }),
                json!({
                    "id": Uuid::new_v4().to_string(),
                    "fieldMetadataId": person_fields[1].id.to_string(),
                    "objectMetadataId": person_id.to_string(),
                    "operand": "CONTAINS",
                    "value": "Active"
                }),
            ],
            ..Default::default()
        };
        let company_rule = crate::permissions::RowLevelRecordsPermissions {
            rule_id: Uuid::new_v4().to_string(),
            can_read_records: true,
            predicates: vec![json!({
                "id": Uuid::new_v4().to_string(),
                "fieldMetadataId": company_status.id.to_string(),
                "objectMetadataId": company_id.to_string(),
                "operand": "IS",
                "value": "ACTIVE"
            })],
            ..Default::default()
        };
        let mut permissions = crate::permissions::RoleRecordsPermissions {
            objects: [
                (person_id.to_string(), Default::default()),
                (company_id.to_string(), Default::default()),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let person_permission = permissions
            .objects
            .get_mut(&person_id.to_string())
            .expect("person permission");
        person_permission.object_metadata_id = person_id.to_string();
        person_permission.row_level_records_permissions = Some(vec![person_rule]);
        person_permission.can_read_some_records = true;
        let company_permission = permissions
            .objects
            .get_mut(&company_id.to_string())
            .expect("company permission");
        company_permission.object_metadata_id = company_id.to_string();
        company_permission.row_level_records_permissions = Some(vec![company_rule]);
        company_permission.can_read_some_records = true;
        let schema = RunnerSchema::build_with_permissions(
            &workspace_id,
            &[person, company_object],
            &fields,
            &FlatEntityMaps::default(),
            permissions,
            None,
        )?;

        sqlx::query(&format!("CREATE SCHEMA {schema_sql}"))
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;
        sqlx::query(&format!(
            "CREATE TABLE {company_table} (id uuid PRIMARY KEY, name text NOT NULL, status text NOT NULL)"
        ))
        .execute(&pool)
        .await
        .map_err(RunnerError::database)?;
        sqlx::query(&format!(
            "CREATE TABLE {person_table} (id uuid PRIMARY KEY, name text NOT NULL, status text NOT NULL, \"companyId\" uuid)"
        ))
        .execute(&pool)
        .await
        .map_err(RunnerError::database)?;
        let visible_company = Uuid::new_v4();
        let hidden_company = Uuid::new_v4();

        for (id, name, status) in [
            (visible_company, "Visible Company", "ACTIVE"),
            (hidden_company, "Hidden Company", "INACTIVE"),
        ] {
            sqlx::query(&format!(
                "INSERT INTO {company_table} (id, name, status) VALUES ($1, $2, $3)"
            ))
            .bind(id)
            .bind(name)
            .bind(status)
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;
        }

        for (id, name, status, related_company) in [
            (Uuid::new_v4(), "Visible Active", "ACTIVE", visible_company),
            (
                Uuid::new_v4(),
                "Visible Inactive",
                "INACTIVE",
                visible_company,
            ),
            (Uuid::new_v4(), "Hidden Active", "ACTIVE", hidden_company),
            (
                Uuid::new_v4(),
                "Hidden Inactive",
                "INACTIVE",
                hidden_company,
            ),
        ] {
            sqlx::query(&format!(
                "INSERT INTO {person_table} (id, name, status, \"companyId\") VALUES ($1, $2, $3, $4)"
            ))
            .bind(id)
            .bind(name)
            .bind(status)
            .bind(related_company)
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;
        }

        let selection = vec![NestedRelationSelection {
            relation_field_name: String::from("company"),
            selected_field_names: vec![
                String::from("id"),
                String::from("name"),
                String::from("status"),
            ],
            first: None,
        }];
        let mut to_one_records = vec![
            json!({ "id": Uuid::new_v4().to_string(), "companyId": visible_company.to_string() }),
            json!({ "id": Uuid::new_v4().to_string(), "companyId": hidden_company.to_string() }),
        ];
        let to_one =
            load_nested_relations(&pool, &schema, "person", &mut to_one_records, &selection).await;
        let mut to_many_records = vec![
            json!({ "id": visible_company.to_string() }),
            json!({ "id": hidden_company.to_string() }),
        ];
        let to_many_selection = vec![NestedRelationSelection {
            relation_field_name: String::from("people"),
            selected_field_names: selection[0].selected_field_names.clone(),
            first: None,
        }];
        let to_many = load_nested_relations(
            &pool,
            &schema,
            "company",
            &mut to_many_records,
            &to_many_selection,
        )
        .await;
        sqlx::query(&format!("DROP SCHEMA {schema_sql} CASCADE"))
            .execute(&pool)
            .await
            .map_err(RunnerError::database)?;
        to_one?;
        to_many?;

        assert_eq!(
            to_one_records[0]["company"]["name"],
            json!("Visible Company")
        );
        assert!(to_one_records[1]["company"].is_null());
        assert_eq!(to_many_records[0]["people"].as_array().unwrap().len(), 1);
        assert_eq!(to_many_records[1]["people"].as_array().unwrap().len(), 1);
        assert_eq!(
            to_many_records[0]["people"][0]["name"],
            json!("Visible Active")
        );
        assert!(to_many_records[0]["people"][0].get("companyId").is_none());
        assert_eq!(
            to_many_records[1]["people"][0]["name"],
            json!("Hidden Active")
        );
        Ok(())
    }

    fn field(object_id: Uuid, name: &str) -> FlatFieldMetadata {
        FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: if name == "id" {
                FieldMetadataType::Uuid
            } else {
                FieldMetadataType::Text
            },
            name: name.to_string(),
            label: name.to_string(),
            ..Default::default()
        }
    }

    fn relation_field(object_id: Uuid, name: &str, target_object_id: Uuid) -> FlatFieldMetadata {
        FlatFieldMetadata {
            id: Uuid::new_v4(),
            object_metadata_id: object_id,
            field_type: FieldMetadataType::Relation,
            name: name.to_string(),
            label: name.to_string(),
            relation_target_object_metadata_id: Some(target_object_id),
            ..Default::default()
        }
    }

    fn object(
        workspace_id: Uuid,
        id: Uuid,
        name: &str,
        fields: &[FlatFieldMetadata],
    ) -> FlatObjectMetadata {
        FlatObjectMetadata {
            id,
            workspace_id,
            name_singular: name.to_string(),
            name_plural: format!("{name}s"),
            label_singular: name.to_string(),
            label_plural: format!("{name}s"),
            target_table_name: Some(name.to_string()),
            field_ids: fields.iter().map(|field| field.id.to_string()).collect(),
            ..Default::default()
        }
    }
}
