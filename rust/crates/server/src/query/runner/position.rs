use serde_json::Value;
use sqlx::Row;

use super::base::ObjectModel;
use super::RunnerError;

const POSITION_KEYWORD_FIRST: &str = "first";

const POSITION_KEYWORD_LAST: &str = "last";

pub async fn override_position_on_records(
    pool: &sqlx::PgPool,
    model: &ObjectModel,
    alias: &str,
    records: &mut [serde_json::Map<String, Value>],
    backfill_undefined: bool,
) -> Result<(), RunnerError> {
    let Some(position_column) = model.shape.column("position") else {
        return Ok(());
    };

    if records.is_empty() {
        return Ok(());
    }

    let first_count = records
        .iter()
        .filter(|record| {
            is_first_position(record, &position_column.column_name, backfill_undefined)
        })
        .count();
    let last_count = records
        .iter()
        .filter(|record| is_last_position(record, &position_column.column_name))
        .count();
    let bounds = if first_count > 0 || last_count > 0 {
        Some(position_bounds(pool, model, alias).await?)
    } else {
        None
    };

    for record in &mut *records {
        let current = record.get(&position_column.column_name);

        if is_first_position(record, &position_column.column_name, backfill_undefined) {
            let bounds = bounds.as_ref().expect("position bounds");
            let index = record
                .get("_positionFirstIndex")
                .and_then(Value::as_i64)
                .unwrap_or_default() as f64;
            record.insert(
                position_column.column_name.clone(),
                serde_json::json!(bounds.min - 1.0 - index),
            );
        } else if is_last_position(record, &position_column.column_name) {
            let bounds = bounds.as_ref().expect("position bounds");
            let index = record
                .get("_positionLastIndex")
                .and_then(Value::as_i64)
                .unwrap_or_default() as f64;
            record.insert(
                position_column.column_name.clone(),
                serde_json::json!(bounds.max + 1.0 + index),
            );
        } else if let Some(Value::Number(number)) = current {
            if number.as_f64().is_none() {
                return Err(RunnerError::invalid_query(
                    "position must be a number, first or last",
                ));
            }
        }
    }

    for record in &mut *records {
        record.remove("_positionFirstIndex");
        record.remove("_positionLastIndex");
    }

    Ok(())
}

fn is_first_position(
    record: &serde_json::Map<String, Value>,
    column_name: &str,
    backfill_undefined: bool,
) -> bool {
    match record.get(column_name) {
        Some(Value::String(value)) => value == POSITION_KEYWORD_FIRST,
        None => backfill_undefined,
        _ => false,
    }
}

fn is_last_position(record: &serde_json::Map<String, Value>, column_name: &str) -> bool {
    matches!(
        record.get(column_name),
        Some(Value::String(value)) if value == POSITION_KEYWORD_LAST
    )
}

struct PositionBounds {
    min: f64,
    max: f64,
}

async fn position_bounds(
    pool: &sqlx::PgPool,
    model: &ObjectModel,
    alias: &str,
) -> Result<PositionBounds, RunnerError> {
    let sql = format!(
        "SELECT min({alias}.\"position\")::double precision AS min, \
         max({alias}.\"position\")::double precision AS max \
         FROM {}.{} AS {alias}",
        crate::query::repository::quote_identifier(&model.shape.schema_name)?,
        crate::query::repository::quote_identifier(&model.shape.table_name)?,
    );
    let row = sqlx::query(&sql)
        .fetch_one(pool)
        .await
        .map_err(RunnerError::database)?;
    let min: Option<f64> = row.try_get("min").map_err(RunnerError::database)?;
    let max: Option<f64> = row.try_get("max").map_err(RunnerError::database)?;

    Ok(PositionBounds {
        min: min.unwrap_or(1.0),
        max: max.unwrap_or(0.0),
    })
}
