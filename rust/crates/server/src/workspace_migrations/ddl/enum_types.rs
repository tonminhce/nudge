#![allow(dead_code)]

use sqlx::PgConnection;

use crate::metadata::names::compute_postgres_enum_name;
use crate::workspace_migrations::column_defs::build_sql_column_definition;
use crate::workspace_migrations::column_defs::WorkspaceSchemaColumnDefinition;

use super::error::DdlError;
use super::escape::{escape_identifier, escape_literal};

const POSTGRES_MAX_IDENTIFIER_LENGTH: usize = 63;

pub fn build_temporary_identifier(base_name: &str, suffix: &str) -> String {
    let max_base_length = POSTGRES_MAX_IDENTIFIER_LENGTH - suffix.len();

    let truncated_base = if base_name.chars().count() <= max_base_length {
        base_name
    } else {
        let safe_boundary = base_name
            .char_indices()
            .nth(max_base_length)
            .map(|(index, _)| index)
            .unwrap_or(base_name.len());

        &base_name[..safe_boundary]
    };

    format!("{truncated_base}{suffix}")
}

pub fn create_enum_sql(
    schema_name: &str,
    enum_name: &str,
    values: &[&str],
) -> Result<String, DdlError> {
    if values.is_empty() {
        return Err(DdlError::enum_operation_failed(
            "Cannot create enum with no values",
        ));
    }

    let mut sanitized_values = Vec::with_capacity(values.len());

    for value in values {
        sanitized_values.push(escape_literal(value)?);
    }

    Ok(format!(
        "CREATE TYPE {}.{} AS ENUM ({})",
        escape_identifier(schema_name)?,
        escape_identifier(enum_name)?,
        sanitized_values.join(", ")
    ))
}

pub async fn create_enum(
    connection: &mut PgConnection,
    schema_name: &str,
    enum_name: &str,
    values: &[&str],
) -> Result<(), DdlError> {
    let sql = create_enum_sql(schema_name, enum_name, values)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn drop_enum_sql(schema_name: &str, enum_name: &str) -> Result<String, DdlError> {
    Ok(format!(
        "DROP TYPE IF EXISTS {}.{}",
        escape_identifier(schema_name)?,
        escape_identifier(enum_name)?
    ))
}

pub async fn drop_enum(
    connection: &mut PgConnection,
    schema_name: &str,
    enum_name: &str,
) -> Result<(), DdlError> {
    let sql = drop_enum_sql(schema_name, enum_name)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn rename_enum_sql(
    schema_name: &str,
    old_enum_name: &str,
    new_enum_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TYPE {}.{} RENAME TO {}",
        escape_identifier(schema_name)?,
        escape_identifier(old_enum_name)?,
        escape_identifier(new_enum_name)?
    ))
}

pub async fn rename_enum(
    connection: &mut PgConnection,
    schema_name: &str,
    old_enum_name: &str,
    new_enum_name: &str,
) -> Result<(), DdlError> {
    let sql = rename_enum_sql(schema_name, old_enum_name, new_enum_name)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn add_enum_value_sql(
    schema_name: &str,
    enum_name: &str,
    value: &str,
    before_value: Option<&str>,
    after_value: Option<&str>,
) -> Result<String, DdlError> {
    let mut sql = format!(
        "ALTER TYPE {}.{} ADD VALUE {}",
        escape_identifier(schema_name)?,
        escape_identifier(enum_name)?,
        escape_literal(value)?
    );

    if let Some(before_value) = before_value.filter(|candidate| !candidate.is_empty()) {
        sql += &format!(" BEFORE {}", escape_literal(before_value)?);
    } else if let Some(after_value) = after_value.filter(|candidate| !candidate.is_empty()) {
        sql += &format!(" AFTER {}", escape_literal(after_value)?);
    }

    Ok(sql)
}

pub async fn add_enum_value(
    connection: &mut PgConnection,
    schema_name: &str,
    enum_name: &str,
    value: &str,
    before_value: Option<&str>,
    after_value: Option<&str>,
) -> Result<(), DdlError> {
    let sql = add_enum_value_sql(schema_name, enum_name, value, before_value, after_value)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn upsert_enum_value_sql(
    schema_name: &str,
    enum_name: &str,
    value: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TYPE {}.{} ADD VALUE IF NOT EXISTS {}",
        escape_identifier(schema_name)?,
        escape_identifier(enum_name)?,
        escape_literal(value)?
    ))
}

pub async fn upsert_enum_value(
    connection: &mut PgConnection,
    schema_name: &str,
    enum_name: &str,
    value: &str,
) -> Result<(), DdlError> {
    let sql = upsert_enum_value_sql(schema_name, enum_name, value)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn rename_enum_value_sql(
    schema_name: &str,
    enum_name: &str,
    old_value: &str,
    new_value: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TYPE {}.{} RENAME VALUE {} TO {}",
        escape_identifier(schema_name)?,
        escape_identifier(enum_name)?,
        escape_literal(old_value)?,
        escape_literal(new_value)?
    ))
}

pub async fn rename_enum_value(
    connection: &mut PgConnection,
    schema_name: &str,
    enum_name: &str,
    old_value: &str,
    new_value: &str,
) -> Result<(), DdlError> {
    let sql = rename_enum_value_sql(schema_name, enum_name, old_value, new_value)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn alter_enum_values_sql_statements(
    schema_name: &str,
    table_name: &str,
    column_definition: &WorkspaceSchemaColumnDefinition,
    enum_values: &[&str],
    old_to_new_enum_option_map: &[(String, String)],
) -> Result<Vec<String>, DdlError> {
    if enum_values.is_empty() {
        return Err(DdlError::enum_operation_failed(format!(
            "Cannot alter enum values for column {} because it has no enum values",
            column_definition.name
        )));
    }

    let column_name = &column_definition.name;
    let enum_name = compute_postgres_enum_name(table_name, column_name);
    let old_enum_name = build_temporary_identifier(&enum_name, "_old");
    let old_column_name = build_temporary_identifier(column_name, "_old");

    let mut statements = vec![
        rename_enum_sql(schema_name, &enum_name, &old_enum_name)?,
        create_enum_sql(schema_name, &enum_name, enum_values)?,
        rename_column_sql(schema_name, table_name, column_name, &old_column_name)?,
        create_column_using_enum_sql(schema_name, table_name, column_definition, &enum_name)?,
    ];

    if !old_to_new_enum_option_map.is_empty() {
        statements.push(migrate_enum_data_sql(
            schema_name,
            table_name,
            &old_column_name,
            column_name,
            old_to_new_enum_option_map,
            &old_enum_name,
            column_definition.is_array,
        )?);
    }

    statements.push(drop_column_sql(schema_name, table_name, &old_column_name)?);
    statements.push(drop_enum_sql(schema_name, &old_enum_name)?);

    Ok(statements)
}

pub async fn alter_enum_values(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    column_definition: &WorkspaceSchemaColumnDefinition,
    enum_values: &[&str],
    old_to_new_enum_option_map: &[(String, String)],
) -> Result<(), DdlError> {
    let statements = alter_enum_values_sql_statements(
        schema_name,
        table_name,
        column_definition,
        enum_values,
        old_to_new_enum_option_map,
    )?;

    for sql in statements {
        sqlx::query(&sql).execute(&mut *connection).await?;
    }

    Ok(())
}

fn rename_column_sql(
    schema_name: &str,
    table_name: &str,
    old_column_name: &str,
    new_column_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TABLE {}.{} RENAME COLUMN {} TO {}",
        escape_identifier(schema_name)?,
        escape_identifier(table_name)?,
        escape_identifier(old_column_name)?,
        escape_identifier(new_column_name)?
    ))
}

fn create_column_using_enum_sql(
    schema_name: &str,
    table_name: &str,
    column_definition: &WorkspaceSchemaColumnDefinition,
    enum_type_name: &str,
) -> Result<String, DdlError> {
    let mut typed_definition = column_definition.clone();
    typed_definition.column_type = format!(
        "{}.{}",
        escape_identifier(schema_name)?,
        escape_identifier(enum_type_name)?
    );

    let column_sql = build_sql_column_definition(&typed_definition)?;

    Ok(format!(
        "ALTER TABLE {}.{} ADD COLUMN {}",
        escape_identifier(schema_name)?,
        escape_identifier(table_name)?,
        column_sql
    ))
}

fn drop_column_sql(
    schema_name: &str,
    table_name: &str,
    column_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TABLE {}.{} DROP COLUMN {}",
        escape_identifier(schema_name)?,
        escape_identifier(table_name)?,
        escape_identifier(column_name)?
    ))
}

fn migrate_enum_data_sql(
    schema_name: &str,
    table_name: &str,
    old_column_name: &str,
    new_column_name: &str,
    old_to_new_enum_option_map: &[(String, String)],
    old_enum_type_name: &str,
    is_array: bool,
) -> Result<String, DdlError> {
    let new_enum_type_name = compute_postgres_enum_name(table_name, new_column_name);

    let escaped_schema = escape_identifier(schema_name)?;
    let escaped_table = escape_identifier(table_name)?;
    let escaped_old_column = escape_identifier(old_column_name)?;
    let escaped_new_column = escape_identifier(new_column_name)?;
    let escaped_new_enum_type = format!(
        "{escaped_schema}.{}",
        escape_identifier(&new_enum_type_name)?
    );

    let mut case_statements = String::new();

    for (index, (old_enum_option, new_enum_option)) in old_to_new_enum_option_map.iter().enumerate()
    {
        if index > 0 {
            case_statements.push(' ');
        }

        case_statements.push_str(&format!(
            "WHEN {} THEN {}::{escaped_new_enum_type}",
            escape_literal(old_enum_option)?,
            escape_literal(new_enum_option)?
        ));
    }

    let mut mapped_values_condition = String::new();

    for (index, (old_value, _)) in old_to_new_enum_option_map.iter().enumerate() {
        if index > 0 {
            mapped_values_condition.push_str(", ");
        }

        mapped_values_condition.push_str(&escape_literal(old_value)?);
    }

    if is_array {
        let escaped_old_enum_type = format!(
            "{escaped_schema}.{}",
            escape_identifier(old_enum_type_name)?
        );

        Ok(format!(
            "\n          UPDATE {escaped_schema}.{escaped_table}\n          SET {escaped_new_column} = (\n            SELECT array_agg(mapped_value) FILTER (WHERE mapped_value IS NOT NULL)\n            FROM (\n              SELECT\n                CASE unnest_value::text\n                  {case_statements}\n                END AS mapped_value\n              FROM unnest({escaped_old_column}) AS unnest_value\n            ) enum_mapping\n          )\n          WHERE {escaped_old_column} IS NOT NULL\n            AND {escaped_old_column} && ARRAY[{mapped_values_condition}]::{escaped_old_enum_type}[]"
        ))
    } else {
        Ok(format!(
            "\n          UPDATE {escaped_schema}.{escaped_table}\n          SET {escaped_new_column} =\n            CASE {escaped_old_column}::text\n              {case_statements}\n            END\n          WHERE {escaped_old_column} IS NOT NULL\n            AND {escaped_old_column}::text IN ({mapped_values_condition})"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage_column() -> WorkspaceSchemaColumnDefinition {
        WorkspaceSchemaColumnDefinition {
            name: "stage".to_string(),
            column_type: "\"workspace_x\".\"wmitem_stage_enum\"".to_string(),
            is_nullable: Some(true),
            default: Some("'NEW'::\"workspace_x\".\"wmitem_stage_enum\"".to_string()),
            is_primary: false,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        }
    }

    #[test]
    fn temporary_identifiers_keep_the_total_length_under_sixty_three() {
        assert_eq!(build_temporary_identifier("stage", "_old"), "stage_old");

        let long_name = "a".repeat(61);
        assert_eq!(
            build_temporary_identifier(&long_name, "_old"),
            format!("{}{}", "a".repeat(59), "_old")
        );

        let exact_name = "a".repeat(59);
        assert_eq!(
            build_temporary_identifier(&exact_name, "_old"),
            format!("{exact_name}_old")
        );
    }

    #[test]
    fn create_enum_sql_quotes_values_and_rejects_empty_lists() {
        assert_eq!(
            create_enum_sql("workspace_x", "stage_enum", &["NEW", "OLD"]).unwrap(),
            "CREATE TYPE \"workspace_x\".\"stage_enum\" AS ENUM ('NEW', 'OLD')"
        );
        assert_eq!(
            create_enum_sql("workspace_x", "stage_enum", &["it's"]).unwrap(),
            "CREATE TYPE \"workspace_x\".\"stage_enum\" AS ENUM ('it''s')"
        );

        let error = create_enum_sql("workspace_x", "stage_enum", &[]).unwrap_err();
        assert_eq!(error.code(), "ENUM_OPERATION_FAILED");
    }

    #[test]
    fn drop_and_rename_enum_sql_match_the_upstream_manager() {
        assert_eq!(
            drop_enum_sql("workspace_x", "stage_enum").unwrap(),
            "DROP TYPE IF EXISTS \"workspace_x\".\"stage_enum\""
        );
        assert_eq!(
            rename_enum_sql("workspace_x", "stage_enum", "stage_enum_old").unwrap(),
            "ALTER TYPE \"workspace_x\".\"stage_enum\" RENAME TO \"stage_enum_old\""
        );
    }

    #[test]
    fn add_enum_value_sql_places_values_before_or_after() {
        assert_eq!(
            add_enum_value_sql("workspace_x", "stage_enum", "DONE", None, None).unwrap(),
            "ALTER TYPE \"workspace_x\".\"stage_enum\" ADD VALUE 'DONE'"
        );
        assert_eq!(
            add_enum_value_sql("workspace_x", "stage_enum", "DONE", Some("NEW"), None).unwrap(),
            "ALTER TYPE \"workspace_x\".\"stage_enum\" ADD VALUE 'DONE' BEFORE 'NEW'"
        );
        assert_eq!(
            add_enum_value_sql("workspace_x", "stage_enum", "DONE", None, Some("NEW")).unwrap(),
            "ALTER TYPE \"workspace_x\".\"stage_enum\" ADD VALUE 'DONE' AFTER 'NEW'"
        );
        assert_eq!(
            add_enum_value_sql("workspace_x", "stage_enum", "DONE", Some(""), Some("NEW")).unwrap(),
            "ALTER TYPE \"workspace_x\".\"stage_enum\" ADD VALUE 'DONE' AFTER 'NEW'"
        );
    }

    #[test]
    fn upsert_and_rename_value_sql_match_the_upstream_manager() {
        assert_eq!(
            upsert_enum_value_sql("workspace_x", "stage_enum", "DONE").unwrap(),
            "ALTER TYPE \"workspace_x\".\"stage_enum\" ADD VALUE IF NOT EXISTS 'DONE'"
        );
        assert_eq!(
            rename_enum_value_sql("workspace_x", "stage_enum", "OLD", "DONE").unwrap(),
            "ALTER TYPE \"workspace_x\".\"stage_enum\" RENAME VALUE 'OLD' TO 'DONE'"
        );
    }

    #[test]
    fn alter_enum_values_emits_the_rename_recreate_backfill_recipe() {
        let statements = alter_enum_values_sql_statements(
            "workspace_x",
            "wmitem",
            &stage_column(),
            &["NEW", "OLD", "DONE"],
            &[("OLD".to_string(), "DONE".to_string())],
        )
        .unwrap();

        assert_eq!(
            statements,
            vec![
                "ALTER TYPE \"workspace_x\".\"wmitem_stage_enum\" RENAME TO \"wmitem_stage_enum_old\"",
                "CREATE TYPE \"workspace_x\".\"wmitem_stage_enum\" AS ENUM ('NEW', 'OLD', 'DONE')",
                "ALTER TABLE \"workspace_x\".\"wmitem\" RENAME COLUMN \"stage\" TO \"stage_old\"",
                "ALTER TABLE \"workspace_x\".\"wmitem\" ADD COLUMN \"stage\" \"workspace_x\".\"wmitem_stage_enum\" DEFAULT 'NEW'::\"workspace_x\".\"wmitem_stage_enum\"",
                "\n          UPDATE \"workspace_x\".\"wmitem\"\n          SET \"stage\" =\n            CASE \"stage_old\"::text\n              WHEN 'OLD' THEN 'DONE'::\"workspace_x\".\"wmitem_stage_enum\"\n            END\n          WHERE \"stage_old\" IS NOT NULL\n            AND \"stage_old\"::text IN ('OLD')",
                "ALTER TABLE \"workspace_x\".\"wmitem\" DROP COLUMN \"stage_old\"",
                "DROP TYPE IF EXISTS \"workspace_x\".\"wmitem_stage_enum_old\""
            ]
        );
    }

    #[test]
    fn alter_enum_values_without_a_map_skips_the_data_migration() {
        let statements = alter_enum_values_sql_statements(
            "workspace_x",
            "wmitem",
            &stage_column(),
            &["NEW"],
            &[],
        )
        .unwrap();

        assert_eq!(statements.len(), 6);
        assert!(!statements
            .iter()
            .any(|statement| statement.contains("UPDATE")));
    }

    #[test]
    fn alter_enum_values_supports_the_array_variant() {
        let mut column = stage_column();
        column.name = "tags".to_string();
        column.is_array = true;
        column.column_type = "\"workspace_x\".\"wmitem_tags_enum\"".to_string();

        let statements = alter_enum_values_sql_statements(
            "workspace_x",
            "wmitem",
            &column,
            &["A", "B"],
            &[("A".to_string(), "B".to_string())],
        )
        .unwrap();

        let migration = &statements[4];

        assert!(migration
            .contains("SELECT array_agg(mapped_value) FILTER (WHERE mapped_value IS NOT NULL)"));
        assert!(migration.contains("FROM unnest(\"tags_old\") AS unnest_value"));
        assert!(migration.contains(
            "AND \"tags_old\" && ARRAY['A']::\"workspace_x\".\"wmitem_tags_enum_old\"[]"
        ));
    }

    #[test]
    fn alter_enum_values_requires_enum_values() {
        let error =
            alter_enum_values_sql_statements("workspace_x", "wmitem", &stage_column(), &[], &[])
                .unwrap_err();

        assert_eq!(error.code(), "ENUM_OPERATION_FAILED");
        assert!(error
            .to_string()
            .contains("Cannot alter enum values for column stage"));
    }

    #[tokio::test]
    async fn alter_enum_values_round_trips_data_when_the_database_is_available() {
        let Some(mut connection) = super::super::test_support::connect().await else {
            return;
        };

        let schema_name = super::super::test_support::unique_schema_name("wm_enum");
        super::super::test_support::create_schema(&mut connection, &schema_name).await;
        super::super::test_support::setup_upstream_extensions(&mut connection).await;

        create_enum(
            &mut connection,
            &schema_name,
            "wmitem_stage_enum",
            &["NEW", "OLD"],
        )
        .await
        .expect("create enum");

        crate::workspace_migrations::ddl::table::create_table(
            &mut connection,
            &schema_name,
            "wmitem",
            &[
                crate::workspace_migrations::column_defs::WorkspaceSchemaColumnDefinition {
                    name: "id".to_string(),
                    column_type: "uuid".to_string(),
                    is_nullable: Some(false),
                    default: Some("public.uuid_generate_v4()".to_string()),
                    is_primary: true,
                    is_array: false,
                    as_expression: None,
                    generated_type: None,
                    is_generated_always_as_identity: false,
                },
            ],
        )
        .await
        .expect("create table");

        crate::workspace_migrations::ddl::column::add_columns(
            &mut connection,
            &schema_name,
            "wmitem",
            &[stage_column_with_schema(&schema_name)],
            false,
        )
        .await
        .expect("add enum column");

        sqlx::query(&format!(
            "INSERT INTO \"{schema_name}\".\"wmitem\"(\"stage\") VALUES ('NEW'), ('OLD')"
        ))
        .execute(&mut connection)
        .await
        .expect("seed rows");

        alter_enum_values(
            &mut connection,
            &schema_name,
            "wmitem",
            &stage_column_with_schema(&schema_name),
            &["NEW", "OLD", "DONE"],
            &[("OLD".to_string(), "DONE".to_string())],
        )
        .await
        .expect("alter enum values");

        let migrated: Vec<(Option<String>,)> = sqlx::query_as(&format!(
            "SELECT \"stage\"::text FROM \"{schema_name}\".\"wmitem\" ORDER BY \"stage\"::text"
        ))
        .fetch_all(&mut connection)
        .await
        .expect("read migrated rows");

        assert_eq!(
            migrated
                .iter()
                .map(|(value,)| value.clone().unwrap_or_default())
                .collect::<Vec<_>>(),
            vec!["DONE".to_string(), "NEW".to_string()]
        );

        super::super::test_support::drop_schema(&mut connection, &schema_name).await;
    }

    fn stage_column_with_schema(schema_name: &str) -> WorkspaceSchemaColumnDefinition {
        WorkspaceSchemaColumnDefinition {
            name: "stage".to_string(),
            column_type: format!("\"{schema_name}\".\"wmitem_stage_enum\""),
            is_nullable: Some(true),
            default: Some(format!("'NEW'::\"{schema_name}\".\"wmitem_stage_enum\"")),
            is_primary: false,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        }
    }
}
