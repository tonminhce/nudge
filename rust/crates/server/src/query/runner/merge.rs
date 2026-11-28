use std::collections::HashMap;
use std::collections::HashSet;

use chrono::SecondsFormat;
use chrono::Utc;
use serde_json::json;
use serde_json::Map;
use serde_json::Value;
use sqlx::PgConnection;
use sqlx::Transaction;
use uuid::Uuid;

use crate::metadata::flat::FieldMetadataType;
use crate::metadata::flat::FlatFieldMetadata;
use crate::metadata::flat::RelationType;
use crate::metadata::names::compute_composite_column_name;
use crate::metadata::names::compute_relation_join_column_name;
use crate::query::repository::quote_identifier;

use super::actors::ActorContext;
use super::base::ObjectModel;
use super::base::RunnerSchema;
use super::base::UniqueField;
use super::find_many::find_many;
use super::find_many::FindManyArgs;
use super::find_many::NestedRelationSelection;
use super::mutation::run_mutation_in_transaction;
use super::mutation::FilteredMutationKind;
use super::RunnerError;
use super::QUERY_MAX_RECORDS;

pub const MUTATION_MAX_MERGE_RECORDS: usize = 9;

#[derive(Debug, Clone)]
pub struct MergeManyArgs {
    pub ids: Vec<Uuid>,
    pub conflict_priority_index: usize,
    pub dry_run: bool,
}

pub async fn merge_many(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    args: &MergeManyArgs,
    actor: Option<&ActorContext>,
    selected_fields: Vec<String>,
    nested: Vec<NestedRelationSelection>,
) -> Result<Value, RunnerError> {
    let _ = selected_fields;
    validate_args(schema, object_name_singular, args)?;
    let model = schema.object(object_name_singular)?;
    if model
        .fields_by_name
        .values()
        .any(|field| !field.is_system && schema.is_field_unreadable(model.object.id, field.id))
    {
        return Err(RunnerError::permission_denied(
            "Merging records with unreadable fields is not allowed",
        ));
    }
    let records =
        fetch_records_to_merge(pool, schema, object_name_singular, model, args, &nested).await?;
    let priority_id = args.ids[args.conflict_priority_index];
    if !records
        .iter()
        .any(|record| record_id(record).is_some_and(|id| id == priority_id))
    {
        return Err(RunnerError::new(
            "RECORD_NOT_FOUND",
            "Priority record not found",
        ));
    }
    if args.dry_run {
        return Ok(dry_run_preview(model, &records, priority_id));
    }
    let ids_to_delete = args
        .ids
        .iter()
        .copied()
        .filter(|id| *id != priority_id)
        .collect::<Vec<_>>();
    let data = merged_write_data(model, &records, priority_id);
    execute_merge(
        pool,
        schema,
        object_name_singular,
        model,
        &ids_to_delete,
        priority_id,
        data,
        actor,
        &nested,
    )
    .await
}

async fn fetch_records_to_merge(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    model: &ObjectModel,
    args: &MergeManyArgs,
    nested: &[NestedRelationSelection],
) -> Result<Vec<Value>, RunnerError> {
    let ids = args
        .ids
        .iter()
        .map(|id| Value::String(id.to_string()))
        .collect::<Vec<_>>();
    let result = find_many(
        pool,
        schema,
        object_name_singular,
        &FindManyArgs {
            filter: Some(json!({"id": {"in": ids}})),
            selected_field_names: readable_field_names(schema, model),
            ..Default::default()
        },
        false,
        if args.dry_run { nested } else { &[] },
    )
    .await?;
    let order = args
        .ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.to_string(), index))
        .collect::<HashMap<_, _>>();
    let mut records = result.records;
    records.sort_by_key(|record| {
        record
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| order.get(id).copied())
            .unwrap_or(usize::MAX)
    });
    if records.len() != args.ids.len() {
        return Err(RunnerError::new(
            "RECORD_NOT_FOUND",
            "One or more records not found",
        ));
    }
    Ok(records)
}

#[expect(clippy::too_many_arguments)]
async fn execute_merge(
    pool: &sqlx::PgPool,
    schema: &RunnerSchema,
    object_name_singular: &str,
    model: &ObjectModel,
    ids_to_delete: &[Uuid],
    priority_id: Uuid,
    data: Map<String, Value>,
    actor: Option<&ActorContext>,
    nested: &[NestedRelationSelection],
) -> Result<Value, RunnerError> {
    let mut transaction = pool.begin().await.map_err(RunnerError::database)?;
    if schema.requires_access_group_recompute(object_name_singular) {
        crate::permissions::access_group::lock_company_access_groups(&mut transaction, schema)
            .await
            .map_err(|error| {
                RunnerError::new("ACCESS_GROUP_RECOMPUTE_FAILED", error.to_string())
            })?;
    }
    repoint_related_records(
        pool,
        &mut transaction,
        schema,
        model,
        ids_to_delete,
        priority_id,
        actor,
    )
    .await?;
    let deleted = run_mutation_in_transaction(
        pool,
        &mut transaction,
        schema,
        object_name_singular,
        FilteredMutationKind::SoftDelete,
        Some(id_filter(ids_to_delete)),
        None,
        actor,
        false,
        vec![String::from("id")],
        false,
    )
    .await?;
    if deleted.records.len() != ids_to_delete.len() {
        return Err(RunnerError::invalid_query(
            "Some records to merge cannot be deleted with the current role",
        ));
    }
    let updated = run_mutation_in_transaction(
        pool,
        &mut transaction,
        schema,
        object_name_singular,
        FilteredMutationKind::Update,
        Some(json!({"id": {"eq": priority_id.to_string()}})),
        Some(Value::Object(data)),
        actor,
        false,
        Vec::new(),
        true,
    )
    .await?;
    let mut records = updated
        .snapshots
        .into_iter()
        .filter_map(|snapshot| snapshot.after)
        .collect::<Vec<_>>();
    if records.is_empty() {
        return Err(RunnerError::new(
            "RECORD_NOT_FOUND",
            "Failed to update record",
        ));
    }
    if schema.requires_access_group_recompute(object_name_singular) {
        let mut affected = ids_to_delete.to_vec();
        affected.push(priority_id);
        crate::permissions::access_group::recompute_after_write(
            pool,
            &mut transaction,
            schema,
            object_name_singular,
            &affected,
        )
        .await
        .map_err(|error| RunnerError::new("ACCESS_GROUP_RECOMPUTE_FAILED", error.to_string()))?;
    }
    transaction.commit().await.map_err(RunnerError::database)?;
    super::nested::load_nested_relations(pool, schema, object_name_singular, &mut records, nested)
        .await?;
    Ok(records.remove(0))
}

async fn repoint_related_records(
    pool: &sqlx::PgPool,
    transaction: &mut Transaction<'_, sqlx::Postgres>,
    schema: &RunnerSchema,
    model: &ObjectModel,
    ids_to_delete: &[Uuid],
    priority_id: Uuid,
    actor: Option<&ActorContext>,
) -> Result<(), RunnerError> {
    if ids_to_delete.is_empty() {
        return Ok(());
    }
    let mut children = Vec::new();
    for child in schema.objects() {
        if child.object.is_remote || child.shape.column("id").is_none() {
            continue;
        }
        for relation in child.shape.relation_shape_by_field_name.values() {
            if relation.relation_type != RelationType::ManyToOne
                || relation.target_object_metadata_id != Some(model.object.id)
            {
                continue;
            }
            let Some(field) = schema.field_metadata(relation.field_metadata_id) else {
                continue;
            };
            if !field.is_active {
                continue;
            }
            let Some(join_column) = relation.join_column_name.as_deref() else {
                continue;
            };
            if child.shape.column(join_column).is_none() {
                continue;
            }
            children.push((child.clone(), join_column.to_string()));
        }
    }
    for (child, join_column) in &children {
        repoint_child_records(
            pool,
            transaction,
            schema,
            child,
            join_column,
            ids_to_delete,
            priority_id,
            actor,
        )
        .await?;
    }
    Ok(())
}

#[expect(clippy::too_many_arguments)]
async fn repoint_child_records(
    pool: &sqlx::PgPool,
    transaction: &mut Transaction<'_, sqlx::Postgres>,
    schema: &RunnerSchema,
    child: &ObjectModel,
    join_column: &str,
    ids_to_delete: &[Uuid],
    priority_id: Uuid,
    actor: Option<&ActorContext>,
) -> Result<(), RunnerError> {
    let groups = duplicate_key_groups(child, join_column);
    let mut columns = Vec::new();
    for group in &groups {
        for column in group {
            if !columns.contains(column) {
                columns.push(column.clone());
            }
        }
    }
    let related =
        select_child_records(transaction, child, join_column, ids_to_delete, &columns).await?;
    if related.is_empty() {
        return Ok(());
    }
    let owned_by_priority = if groups.is_empty() {
        Vec::new()
    } else {
        select_child_records(transaction, child, join_column, &[priority_id], &columns).await?
    };
    let mut taken = owned_by_priority
        .iter()
        .flat_map(|record| duplicate_keys(record, &groups))
        .collect::<HashSet<_>>();
    let mut ids_to_migrate = Vec::new();
    let mut ids_to_soft_delete = Vec::new();
    for record in &related {
        let Some(id) = map_record_id(record) else {
            continue;
        };
        let keys = duplicate_keys(record, &groups);
        if keys.iter().any(|key| taken.contains(key)) {
            ids_to_soft_delete.push(id);
            continue;
        }
        taken.extend(keys);
        ids_to_migrate.push(id);
    }
    let child_name = child.object.name_singular.as_str();
    for chunk in ids_to_soft_delete.chunks(QUERY_MAX_RECORDS) {
        run_mutation_in_transaction(
            pool,
            transaction,
            schema,
            child_name,
            FilteredMutationKind::SoftDelete,
            Some(id_filter(chunk)),
            None,
            actor,
            false,
            vec![String::from("id")],
            false,
        )
        .await?;
    }
    for chunk in ids_to_migrate.chunks(QUERY_MAX_RECORDS) {
        let mut data = Map::new();
        data.insert(
            join_column.to_string(),
            Value::String(priority_id.to_string()),
        );
        run_mutation_in_transaction(
            pool,
            transaction,
            schema,
            child_name,
            FilteredMutationKind::Update,
            Some(id_filter(chunk)),
            Some(Value::Object(data)),
            actor,
            false,
            vec![String::from("id")],
            false,
        )
        .await?;
    }
    Ok(())
}

async fn select_child_records(
    transaction: &mut PgConnection,
    child: &ObjectModel,
    join_column: &str,
    ids: &[Uuid],
    columns: &[String],
) -> Result<Vec<Map<String, Value>>, RunnerError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let alias = child.object.name_singular.as_str();
    let alias_sql = quote_identifier(alias)?;
    let table = format!(
        "{}.{}",
        quote_identifier(&child.shape.schema_name)?,
        quote_identifier(&child.shape.table_name)?
    );
    let mut selected = vec![format!("{alias_sql}.{}", quote_identifier("id")?)];
    for column in columns {
        if column == "id" {
            continue;
        }
        selected.push(format!("{alias_sql}.{}", quote_identifier(column)?));
    }
    let mut predicates = format!(
        "{alias_sql}.{} = ANY($1::uuid[])",
        quote_identifier(join_column)?
    );
    if child.shape.has_deleted_at_column {
        predicates.push_str(&format!(
            " AND {alias_sql}.{} IS NULL",
            quote_identifier("deletedAt")?
        ));
    }
    let sql = format!(
        "SELECT {} FROM {table} AS {alias_sql} WHERE {predicates} ORDER BY {alias_sql}.{}",
        selected.join(", "),
        quote_identifier("id")?,
    );
    let rows = sqlx::query(&sql)
        .bind(ids.to_vec())
        .fetch_all(&mut *transaction)
        .await
        .map_err(RunnerError::database)?;
    rows.iter().map(super::base::row_to_json).collect()
}

fn duplicate_key_groups(child: &ObjectModel, join_column: &str) -> Vec<Vec<String>> {
    child
        .unique_field_groups
        .iter()
        .filter_map(|group| unique_column_group(child, group))
        .filter(|columns| columns.iter().any(|column| column == join_column))
        .map(|columns| {
            columns
                .into_iter()
                .filter(|column| column != join_column)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn unique_column_group(child: &ObjectModel, group: &[UniqueField]) -> Option<Vec<String>> {
    let mut columns = Vec::new();
    for unique in group {
        let field = child.fields_by_name.get(&unique.field_name)?;
        let column = if matches!(
            field.field_type,
            FieldMetadataType::Relation | FieldMetadataType::MorphRelation
        ) {
            compute_relation_join_column_name(&field.name)
        } else if let Some(sub_field_name) = unique.sub_field_name.as_deref() {
            compute_composite_column_name(&field.name, sub_field_name)
        } else {
            field.name.clone()
        };
        child.shape.column(&column)?;
        columns.push(column);
    }
    Some(columns)
}

fn duplicate_keys(record: &Map<String, Value>, groups: &[Vec<String>]) -> Vec<String> {
    groups
        .iter()
        .filter(|group| {
            group
                .iter()
                .all(|column| record.get(column).is_some_and(|value| !value.is_null()))
        })
        .map(|group| {
            let values = group
                .iter()
                .map(|column| record.get(column).cloned().unwrap_or(Value::Null))
                .collect::<Vec<_>>();
            json!([group, values]).to_string()
        })
        .collect()
}

fn id_filter(ids: &[Uuid]) -> Value {
    json!({
        "id": {
            "in": ids.iter().map(|id| Value::String(id.to_string())).collect::<Vec<_>>()
        }
    })
}

fn dry_run_preview(model: &ObjectModel, records: &[Value], priority_id: Uuid) -> Value {
    let mut preview = records
        .iter()
        .find(|record| record_id(record).is_some_and(|id| id == priority_id))
        .cloned()
        .unwrap_or(Value::Null);
    let merged = deep_merge(model, records, priority_id, true);
    if let Some(object) = preview.as_object_mut() {
        for (field_name, value) in merged {
            object.insert(field_name, value);
        }
        object.insert(
            String::from("id"),
            Value::String(Uuid::new_v4().to_string()),
        );
        object.insert(
            String::from("deletedAt"),
            Value::String(Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)),
        );
    }
    preview
}

fn merged_write_data(
    model: &ObjectModel,
    records: &[Value],
    priority_id: Uuid,
) -> Map<String, Value> {
    let mut data = Map::new();
    for (field_name, value) in deep_merge(model, records, priority_id, false) {
        let Some(field) = model.fields_by_name.get(&field_name) else {
            continue;
        };
        let Some(column) = write_column(model, field) else {
            continue;
        };
        data.insert(column, value);
    }
    data
}

fn deep_merge(
    model: &ObjectModel,
    records: &[Value],
    priority_id: Uuid,
    dry_run: bool,
) -> Vec<(String, Value)> {
    let mut merged = Vec::new();
    for field in model.fields_by_name.values() {
        if field.is_system {
            continue;
        }
        let present = records
            .iter()
            .filter_map(|record| {
                let id = record_id(record)?;
                let value = record.get(&field.name).cloned().unwrap_or(Value::Null);
                has_record_field_value(&value).then_some((value, id))
            })
            .collect::<Vec<_>>();
        let relation_type = model
            .shape
            .relation_shape_by_field_name
            .get(&field.name)
            .map(|relation| relation.relation_type);
        if let Some(value) = merge_field_value(field, &present, priority_id, dry_run, relation_type)
        {
            merged.push((field.name.clone(), value));
        }
    }
    merged
}

fn merge_field_value(
    field: &FlatFieldMetadata,
    present: &[(Value, Uuid)],
    priority_id: Uuid,
    dry_run: bool,
    relation_type: Option<RelationType>,
) -> Option<Value> {
    if present.is_empty() {
        return None;
    }
    if present.len() == 1 {
        return Some(present[0].0.clone());
    }
    let values = present
        .iter()
        .map(|(value, _)| value.clone())
        .collect::<Vec<_>>();
    match field.field_type {
        FieldMetadataType::Array | FieldMetadataType::MultiSelect => {
            Some(merge_array_values(&values))
        }
        FieldMetadataType::Emails => Some(merge_emails(present, priority_id)),
        FieldMetadataType::Phones => Some(merge_phones(present, priority_id)),
        FieldMetadataType::Links => Some(merge_links(present, priority_id)),
        FieldMetadataType::Relation if dry_run => Some(merge_relation_for_dry_run(
            present,
            priority_id,
            relation_type,
        )),
        _ => Some(default_merge_value(present, priority_id)),
    }
}

fn write_column(model: &ObjectModel, field: &FlatFieldMetadata) -> Option<String> {
    if matches!(
        field.field_type,
        FieldMetadataType::Relation | FieldMetadataType::MorphRelation
    ) {
        let column = compute_relation_join_column_name(&field.name);
        return model.shape.column(&column).map(|_| column);
    }
    if model.shape.column(&field.name).is_some() {
        return Some(field.name.clone());
    }
    model
        .shape
        .columns
        .iter()
        .any(|column| column.composite_parent_field_name.as_deref() == Some(field.name.as_str()))
        .then(|| field.name.clone())
}

fn ordered_records(records: &[Value], priority_id: Uuid) -> Vec<&Value> {
    let mut ordered = Vec::with_capacity(records.len());
    if let Some(record) = records
        .iter()
        .find(|record| record_id(record).is_some_and(|id| id == priority_id))
    {
        ordered.push(record);
    }
    ordered.extend(
        records
            .iter()
            .filter(|record| record_id(record).is_none_or(|id| id != priority_id)),
    );
    ordered
}

fn default_merge_value(present: &[(Value, Uuid)], priority_id: Uuid) -> Value {
    present
        .iter()
        .find(|(_, id)| *id == priority_id)
        .filter(|(value, _)| has_record_field_value(value))
        .or_else(|| {
            present
                .iter()
                .find(|(value, _)| has_record_field_value(value))
        })
        .map(|(value, _)| value.clone())
        .unwrap_or(Value::Null)
}

fn merge_array_values(values: &[Value]) -> Value {
    let merged = stable_union(
        values
            .iter()
            .filter_map(Value::as_array)
            .flatten()
            .filter(|value| has_record_field_value(value))
            .cloned(),
    );
    if merged.is_empty() {
        Value::Null
    } else {
        Value::Array(merged)
    }
}

fn merge_relation_for_dry_run(
    present: &[(Value, Uuid)],
    priority_id: Uuid,
    relation_type: Option<RelationType>,
) -> Value {
    if relation_type != Some(RelationType::OneToMany) {
        return default_merge_value(present, priority_id);
    }
    let mut related: Vec<(String, Value)> = Vec::new();
    for (value, _) in present {
        let Some(items) = value.as_array() else {
            continue;
        };
        for item in items {
            let Some(id) = item.get("id").and_then(Value::as_str) else {
                continue;
            };
            match related.iter_mut().find(|(existing, _)| existing == id) {
                Some(entry) => entry.1 = item.clone(),
                None => related.push((id.to_string(), item.clone())),
            }
        }
    }
    Value::Array(related.into_iter().map(|(_, item)| item).collect())
}

fn stable_union(values: impl IntoIterator<Item = Value>) -> Vec<Value> {
    let mut output = Vec::new();
    for value in values {
        if !output.contains(&value) {
            output.push(value);
        }
    }
    output
}

fn merge_emails(present: &[(Value, Uuid)], priority_id: Uuid) -> Value {
    let primary = meaningful_email(present, priority_id);
    let mut emails = Vec::new();
    for (value, _) in present {
        if let Some(email) = meaningful_string(value.get("primaryEmail")) {
            push_unique(&mut emails, email);
        }
        if let Some(items) = array_or_json_values(value.get("additionalEmails")) {
            for item in items {
                if let Some(email) = item.as_str().and_then(meaningful_text) {
                    push_unique(&mut emails, email);
                }
            }
        }
    }
    emails.retain(|email| email != &primary);
    json!({
        "primaryEmail": primary,
        "additionalEmails": if emails.is_empty() { Value::Null } else { json!(emails) },
    })
}

fn array_or_json_values(value: Option<&Value>) -> Option<Vec<Value>> {
    match value {
        Some(Value::Array(items)) => Some(items.clone()),
        Some(Value::String(raw)) => serde_json::from_str::<Value>(raw)
            .ok()
            .and_then(|parsed| parsed.as_array().cloned()),
        _ => None,
    }
}

fn meaningful_email(present: &[(Value, Uuid)], priority_id: Uuid) -> String {
    present
        .iter()
        .find(|(_, id)| *id == priority_id)
        .and_then(|(value, _)| meaningful_string(value.get("primaryEmail")))
        .or_else(|| {
            present
                .iter()
                .find_map(|(value, _)| meaningful_string(value.get("primaryEmail")))
        })
        .unwrap_or_default()
}

fn meaningful_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).and_then(meaningful_text)
}

fn meaningful_text(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_string())
}

fn push_unique(output: &mut Vec<String>, value: String) {
    if !output.contains(&value) {
        output.push(value);
    }
}

fn merge_phones(present: &[(Value, Uuid)], priority_id: Uuid) -> Value {
    let source = present
        .iter()
        .find(|(value, id)| {
            *id == priority_id && meaningful_string(value.get("primaryPhoneNumber")).is_some()
        })
        .or_else(|| {
            present
                .iter()
                .find(|(value, _)| meaningful_string(value.get("primaryPhoneNumber")).is_some())
        })
        .map(|(value, _)| value.clone());
    let mut phones = Vec::new();
    for (value, _) in present {
        if let Some(number) = meaningful_string(value.get("primaryPhoneNumber")) {
            push_phone_unique(
                &mut phones,
                number,
                value
                    .get("primaryPhoneCountryCode")
                    .cloned()
                    .unwrap_or(Value::Null),
                value
                    .get("primaryPhoneCallingCode")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        if let Some(items) = array_or_json_values(value.get("additionalPhones")) {
            for item in items {
                if let Some(number) = meaningful_string(item.get("number")) {
                    push_phone_unique(
                        &mut phones,
                        number,
                        item.get("countryCode").cloned().unwrap_or(Value::Null),
                        item.get("callingCode").cloned().unwrap_or(Value::Null),
                    );
                }
            }
        }
    }
    let primary = source
        .as_ref()
        .and_then(|value| meaningful_string(value.get("primaryPhoneNumber")));
    if let Some(primary) = primary.as_ref() {
        phones.retain(|phone| phone["number"].as_str() != Some(primary.as_str()));
    }
    json!({
        "primaryPhoneNumber": primary.unwrap_or_default(),
        "primaryPhoneCountryCode": composite_source_value(source.as_ref(), "primaryPhoneCountryCode", Value::Null),
        "primaryPhoneCallingCode": composite_source_value(source.as_ref(), "primaryPhoneCallingCode", Value::String(String::new())),
        "additionalPhones": if phones.is_empty() { Value::Null } else { Value::Array(phones) },
    })
}

fn push_phone_unique(
    output: &mut Vec<Value>,
    number: String,
    country_code: Value,
    calling_code: Value,
) {
    if output
        .iter()
        .any(|value| value["number"].as_str() == Some(number.as_str()))
    {
        return;
    }
    output.push(json!({
        "number": number,
        "countryCode": country_code,
        "callingCode": calling_code,
    }));
}

fn merge_links(present: &[(Value, Uuid)], priority_id: Uuid) -> Value {
    let source = present
        .iter()
        .find(|(value, id)| {
            *id == priority_id && meaningful_string(value.get("primaryLinkUrl")).is_some()
        })
        .or_else(|| {
            present
                .iter()
                .find(|(value, _)| meaningful_string(value.get("primaryLinkUrl")).is_some())
        })
        .map(|(value, _)| value.clone());
    let mut links = Vec::new();
    for (value, _) in present {
        if let Some(url) = meaningful_string(value.get("primaryLinkUrl")) {
            push_link_unique(
                &mut links,
                url,
                value
                    .get("primaryLinkLabel")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        if let Some(items) = array_or_json_values(value.get("secondaryLinks")) {
            for item in items {
                if let Some(url) = meaningful_string(item.get("url")) {
                    push_link_unique(
                        &mut links,
                        url,
                        item.get("label").cloned().unwrap_or(Value::Null),
                    );
                }
            }
        }
    }
    let primary = source
        .as_ref()
        .and_then(|value| meaningful_string(value.get("primaryLinkUrl")));
    if let Some(primary) = primary.as_ref() {
        links.retain(|link| link["url"].as_str() != Some(primary.as_str()));
    }
    json!({
        "primaryLinkUrl": primary.unwrap_or_default(),
        "primaryLinkLabel": composite_source_value(source.as_ref(), "primaryLinkLabel", Value::String(String::new())),
        "secondaryLinks": if links.is_empty() { Value::Null } else { Value::Array(links) },
    })
}

fn composite_source_value(source: Option<&Value>, key: &str, missing_source: Value) -> Value {
    match source {
        Some(value) => value.get(key).cloned().unwrap_or(Value::Null),
        None => missing_source,
    }
}

fn push_link_unique(output: &mut Vec<Value>, url: String, label: Value) {
    if output
        .iter()
        .any(|value| value["url"].as_str() == Some(url.as_str()))
    {
        return;
    }
    output.push(json!({"url": url, "label": label}));
}

fn validate_args(
    schema: &RunnerSchema,
    object_name_singular: &str,
    args: &MergeManyArgs,
) -> Result<(), RunnerError> {
    let object = schema.object(object_name_singular)?;
    if object.object.is_remote {
        return Err(RunnerError::new(
            "OBJECT_MUTATION_NOT_ALLOWED",
            "Remote objects are read-only",
        ));
    }
    if object.object.is_system {
        return Err(RunnerError::invalid_query(format!(
            "Merge is not available on system object '{object_name_singular}'."
        )));
    }
    if args.ids.len() < 2 {
        return Err(RunnerError::invalid_query(
            "At least 2 record IDs are required for merge",
        ));
    }
    if args.ids.len() > MUTATION_MAX_MERGE_RECORDS {
        return Err(RunnerError::invalid_query(format!(
            "Maximum {MUTATION_MAX_MERGE_RECORDS} records can be merged at once"
        )));
    }
    if args.conflict_priority_index >= args.ids.len() {
        return Err(RunnerError::invalid_query(format!(
            "Invalid conflict priority '{}'. Valid options for {} records: 0-{}",
            args.conflict_priority_index,
            args.ids.len(),
            args.ids.len() - 1
        )));
    }
    Ok(())
}

pub(crate) fn readable_field_names(
    schema: &RunnerSchema,
    model: &super::base::ObjectModel,
) -> Vec<String> {
    let mut names = Vec::new();
    for column in &model.shape.columns {
        if schema.is_field_unreadable(model.object.id, column.field_metadata_id) {
            continue;
        }
        if !names.contains(&column.field_name) {
            names.push(column.field_name.clone());
        }
    }
    if !names.iter().any(|name| name == "id") {
        names.insert(0, String::from("id"));
    }
    names
}

fn record_id(record: &Value) -> Option<Uuid> {
    record
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
}

fn map_record_id(record: &Map<String, Value>) -> Option<Uuid> {
    record
        .get("id")
        .and_then(Value::as_str)
        .and_then(|id| Uuid::parse_str(id).ok())
}

fn has_record_field_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(values) => !values.is_empty(),
        Value::Object(values) => values.values().any(has_record_field_value),
        Value::Bool(_) => true,
        Value::Number(value) => value.as_f64().is_some_and(|value| !value.is_nan()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pair(value: Value, id: Uuid) -> (Value, Uuid) {
        (value, id)
    }

    #[test]
    fn composite_merges_keep_priority_primaries_and_union_extras() {
        let priority = Uuid::new_v4();
        let other = Uuid::new_v4();
        let present = vec![
            pair(
                json!({
                    "primaryEmail": "first@example.com",
                    "additionalEmails": ["shared@example.com"],
                    "primaryPhoneNumber": "111",
                    "primaryPhoneCountryCode": "US",
                    "primaryPhoneCallingCode": "+1",
                    "additionalPhones": [{"number": "222", "countryCode": "US", "callingCode": "+1"}],
                    "primaryLinkUrl": "https://first.example.com",
                    "primaryLinkLabel": "First",
                    "secondaryLinks": [{"url": "https://shared.example.com", "label": "Shared"}]
                }),
                priority,
            ),
            pair(
                json!({
                    "primaryEmail": "second@example.com",
                    "additionalEmails": ["shared@example.com", "third@example.com"],
                    "primaryPhoneNumber": "333",
                    "primaryPhoneCountryCode": "GB",
                    "primaryPhoneCallingCode": "+44",
                    "additionalPhones": [{"number": "222", "countryCode": "GB", "callingCode": "+44"}],
                    "primaryLinkUrl": "https://second.example.com",
                    "primaryLinkLabel": "Second",
                    "secondaryLinks": [{"url": "https://shared.example.com", "label": "Other"}]
                }),
                other,
            ),
        ];

        assert_eq!(
            merge_emails(&present, priority),
            json!({"primaryEmail": "first@example.com", "additionalEmails": ["shared@example.com", "second@example.com", "third@example.com"]})
        );
        assert_eq!(
            merge_phones(&present, priority),
            json!({
                "primaryPhoneNumber": "111",
                "primaryPhoneCountryCode": "US",
                "primaryPhoneCallingCode": "+1",
                "additionalPhones": [
                    {"number": "222", "countryCode": "US", "callingCode": "+1"},
                    {"number": "333", "countryCode": "GB", "callingCode": "+44"}
                ]
            })
        );
        assert_eq!(
            merge_links(&present, priority),
            json!({
                "primaryLinkUrl": "https://first.example.com",
                "primaryLinkLabel": "First",
                "secondaryLinks": [
                    {"url": "https://shared.example.com", "label": "Shared"},
                    {"url": "https://second.example.com", "label": "Second"}
                ]
            })
        );
    }

    #[test]
    fn composite_merges_take_the_primary_from_the_priority_record() {
        let priority = Uuid::new_v4();
        let other = Uuid::new_v4();
        let present = vec![
            pair(json!({"primaryEmail": "other@example.com"}), other),
            pair(json!({"primaryEmail": "kept@example.com"}), priority),
        ];

        assert_eq!(
            merge_emails(&present, priority),
            json!({"primaryEmail": "kept@example.com", "additionalEmails": ["other@example.com"]})
        );
        assert_eq!(
            merge_emails(&present, other),
            json!({"primaryEmail": "other@example.com", "additionalEmails": ["kept@example.com"]})
        );
    }

    #[test]
    fn composite_merges_return_null_additions_when_nothing_remains() {
        let priority = Uuid::new_v4();
        let other = Uuid::new_v4();

        assert_eq!(
            merge_emails(
                &[
                    pair(json!({"primaryEmail": "same@example.com"}), priority),
                    pair(json!({"primaryEmail": "same@example.com"}), other),
                ],
                priority
            ),
            json!({"primaryEmail": "same@example.com", "additionalEmails": Value::Null})
        );
        assert_eq!(
            merge_phones(
                &[
                    pair(
                        json!({
                            "primaryPhoneNumber": "111",
                            "primaryPhoneCountryCode": "US",
                            "primaryPhoneCallingCode": Value::Null
                        }),
                        priority
                    ),
                    pair(
                        json!({
                            "primaryPhoneNumber": "111",
                            "primaryPhoneCountryCode": "US",
                            "primaryPhoneCallingCode": Value::Null
                        }),
                        other
                    ),
                ],
                priority
            ),
            json!({
                "primaryPhoneNumber": "111",
                "primaryPhoneCountryCode": "US",
                "primaryPhoneCallingCode": Value::Null,
                "additionalPhones": Value::Null
            })
        );
        assert_eq!(
            merge_links(
                &[
                    pair(
                        json!({"primaryLinkUrl": "https://a.example.com", "primaryLinkLabel": "A"}),
                        priority
                    ),
                    pair(
                        json!({"primaryLinkUrl": "https://a.example.com", "primaryLinkLabel": "A"}),
                        other
                    ),
                ],
                priority
            ),
            json!({
                "primaryLinkUrl": "https://a.example.com",
                "primaryLinkLabel": "A",
                "secondaryLinks": Value::Null
            })
        );
        assert_eq!(merge_array_values(&[json!([""]), json!([""])]), Value::Null);
    }

    #[test]
    fn scalar_merge_prefers_the_priority_record_and_keeps_zero_and_false() {
        let priority = Uuid::new_v4();
        let other = Uuid::new_v4();

        assert_eq!(
            default_merge_value(&[pair(json!(0), other), pair(json!(2), priority)], priority),
            json!(2)
        );
        assert_eq!(
            default_merge_value(
                &[pair(json!(false), priority), pair(json!(true), other)],
                priority
            ),
            json!(false)
        );
        assert_eq!(
            default_merge_value(
                &[
                    pair(json!("fallback"), other),
                    pair(Value::String(String::from(" ")), priority)
                ],
                priority
            ),
            json!("fallback")
        );
        assert_eq!(
            merge_relation_for_dry_run(
                &[pair(json!(2), other), pair(json!(7), priority)],
                priority,
                Some(RelationType::ManyToOne)
            ),
            json!(7)
        );
        assert_eq!(
            merge_relation_for_dry_run(
                &[
                    pair(json!([{"id": "one"}, {"id": "two"}]), other),
                    pair(json!([{"id": "two"}]), priority),
                ],
                priority,
                Some(RelationType::OneToMany)
            ),
            json!([{"id": "one"}, {"id": "two"}])
        );
        assert_eq!(
            merge_relation_for_dry_run(
                &[
                    pair(json!({"id": "lost"}), other),
                    pair(Value::Null, priority)
                ],
                priority,
                Some(RelationType::ManyToOne)
            ),
            json!({"id": "lost"})
        );
    }

    #[test]
    fn merge_validation_reports_the_upstream_input_errors() {
        let schema = validation_schema();
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let too_many = (0..MUTATION_MAX_MERGE_RECORDS + 1)
            .map(|_| Uuid::new_v4())
            .collect::<Vec<_>>();

        assert_eq!(
            error_of(validate_args(
                &schema,
                "item",
                &MergeManyArgs {
                    ids: vec![ids[0]],
                    conflict_priority_index: 0,
                    dry_run: false
                }
            )),
            (
                "INVALID_QUERY_INPUT".to_string(),
                "At least 2 record IDs are required for merge".to_string()
            )
        );
        assert_eq!(
            error_of(validate_args(
                &schema,
                "item",
                &MergeManyArgs {
                    ids: too_many,
                    conflict_priority_index: 0,
                    dry_run: false
                }
            )),
            (
                "INVALID_QUERY_INPUT".to_string(),
                "Maximum 9 records can be merged at once".to_string()
            )
        );
        assert_eq!(
            error_of(validate_args(
                &schema,
                "item",
                &MergeManyArgs {
                    ids: ids.clone(),
                    conflict_priority_index: 2,
                    dry_run: false
                }
            )),
            (
                "INVALID_QUERY_INPUT".to_string(),
                "Invalid conflict priority '2'. Valid options for 2 records: 0-1".to_string()
            )
        );
        assert_eq!(
            error_of(validate_args(
                &schema,
                "remoteItem",
                &MergeManyArgs {
                    ids: ids.clone(),
                    conflict_priority_index: 0,
                    dry_run: false
                }
            )),
            (
                "OBJECT_MUTATION_NOT_ALLOWED".to_string(),
                "Remote objects are read-only".to_string()
            )
        );
        assert_eq!(
            error_of(validate_args(
                &schema,
                "systemItem",
                &MergeManyArgs {
                    ids,
                    conflict_priority_index: 0,
                    dry_run: false
                }
            )),
            (
                "INVALID_QUERY_INPUT".to_string(),
                "Merge is not available on system object 'systemItem'.".to_string()
            )
        );
        assert!(validate_args(
            &schema,
            "item",
            &MergeManyArgs {
                ids: vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()],
                conflict_priority_index: 2,
                dry_run: true
            }
        )
        .is_ok());
    }

    fn error_of(result: Result<(), RunnerError>) -> (String, String) {
        let error = result.expect_err("merge validation error");
        (error.code().to_string(), error.message().to_string())
    }

    fn validation_schema() -> RunnerSchema {
        let workspace_id = Uuid::new_v4();
        let mut fields = crate::metadata::flat::FlatEntityMaps::default();
        let mut objects = Vec::new();

        for (name, is_remote, is_system) in [
            ("item", false, false),
            ("remoteItem", true, false),
            ("systemItem", false, true),
        ] {
            let object_id = Uuid::new_v4();
            let id_field = crate::metadata::flat::FlatFieldMetadata {
                id: Uuid::new_v4(),
                object_metadata_id: object_id,
                field_type: crate::metadata::flat::FieldMetadataType::Uuid,
                name: String::from("id"),
                label: String::from("id"),
                ..Default::default()
            };
            fields.add(
                id_field.id.to_string(),
                id_field.clone(),
                id_field.id.to_string(),
                workspace_id.to_string(),
            );
            objects.push(crate::metadata::flat::FlatObjectMetadata {
                id: object_id,
                workspace_id,
                name_singular: name.to_string(),
                name_plural: format!("{name}s"),
                label_singular: name.to_string(),
                label_plural: format!("{name}s"),
                target_table_name: Some(name.to_string()),
                field_ids: vec![id_field.id.to_string()],
                is_remote,
                is_system,
                ..Default::default()
            });
        }

        RunnerSchema::build(
            &workspace_id,
            &objects,
            &fields,
            &crate::metadata::flat::FlatEntityMaps::default(),
        )
        .expect("merge validation schema")
    }

    #[test]
    fn has_record_field_value_matches_the_upstream_meaningfulness_rule() {
        assert!(!has_record_field_value(&json!("  ")));
        assert!(!has_record_field_value(
            &json!({"primaryEmail": "", "additionalEmails": []})
        ));
        assert!(has_record_field_value(
            &json!({"primaryEmail": "", "additionalEmails": ["a@b.com"]})
        ));
        assert!(has_record_field_value(&json!(0)));
        assert!(has_record_field_value(&json!(false)));
    }

    #[test]
    fn duplicate_keys_only_form_when_every_column_is_defined() {
        let groups = vec![vec![String::from("domain")], vec![String::from("missing")]];
        let record = json!({"domain": "acme.com", "missing": Value::Null})
            .as_object()
            .cloned()
            .expect("record object");

        assert_eq!(
            duplicate_keys(&record, &groups),
            vec!["[[\"domain\"],[\"acme.com\"]]"]
        );
        assert!(duplicate_keys(&Map::new(), &groups).is_empty());
    }
}
