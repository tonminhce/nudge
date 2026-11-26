#![allow(dead_code)]

use sqlx::PgConnection;

use crate::workspace_migrations::column_defs::build_sql_column_definition;
use crate::workspace_migrations::column_defs::WorkspaceSchemaColumnDefinition;

use super::error::DdlError;
use super::escape::escape_identifier;

#[derive(Debug, Clone, PartialEq)]
pub enum AlterColumnDefault {
    Drop,
    SetNull,
    Literal(String),
}

pub fn add_columns_sql(
    schema_name: &str,
    table_name: &str,
    column_definitions: &[WorkspaceSchemaColumnDefinition],
    should_skip_existing_columns: bool,
) -> Result<String, DdlError> {
    if column_definitions.is_empty() {
        return Ok(String::new());
    }

    let if_not_exists_clause = if should_skip_existing_columns {
        "IF NOT EXISTS "
    } else {
        ""
    };

    let mut add_column_clauses = Vec::with_capacity(column_definitions.len());

    for column in column_definitions {
        add_column_clauses.push(format!(
            "ADD COLUMN {if_not_exists_clause}{}",
            build_sql_column_definition(column)?
        ));
    }

    Ok(format!(
        "ALTER TABLE {} {}",
        qualified_table(schema_name, table_name)?,
        add_column_clauses.join(", ")
    ))
}

pub async fn add_columns(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    column_definitions: &[WorkspaceSchemaColumnDefinition],
    should_skip_existing_columns: bool,
) -> Result<(), DdlError> {
    if column_definitions.is_empty() {
        return Ok(());
    }

    let sql = add_columns_sql(
        schema_name,
        table_name,
        column_definitions,
        should_skip_existing_columns,
    )?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn drop_columns_sql(
    schema_name: &str,
    table_name: &str,
    column_names: &[&str],
    cascade: bool,
) -> Result<String, DdlError> {
    if column_names.is_empty() {
        return Ok(String::new());
    }

    let cascade_clause = if cascade { " CASCADE" } else { "" };

    let mut drop_clauses = Vec::with_capacity(column_names.len());

    for name in column_names {
        drop_clauses.push(format!(
            "DROP COLUMN IF EXISTS {}{cascade_clause}",
            escape_identifier(name)?
        ));
    }

    Ok(format!(
        "ALTER TABLE {} {}",
        qualified_table(schema_name, table_name)?,
        drop_clauses.join(", ")
    ))
}

pub async fn drop_columns(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    column_names: &[&str],
    cascade: bool,
) -> Result<(), DdlError> {
    if column_names.is_empty() {
        return Ok(());
    }

    let sql = drop_columns_sql(schema_name, table_name, column_names, cascade)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn rename_column_sql(
    schema_name: &str,
    table_name: &str,
    old_column_name: &str,
    new_column_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TABLE {} RENAME COLUMN {} TO {}",
        qualified_table(schema_name, table_name)?,
        escape_identifier(old_column_name)?,
        escape_identifier(new_column_name)?
    ))
}

pub async fn rename_column(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    old_column_name: &str,
    new_column_name: &str,
) -> Result<(), DdlError> {
    let sql = rename_column_sql(schema_name, table_name, old_column_name, new_column_name)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn alter_column_default_sql(
    schema_name: &str,
    table_name: &str,
    column_name: &str,
    default: AlterColumnDefault,
) -> Result<String, DdlError> {
    let table_ref = qualified_table(schema_name, table_name)?;
    let column_ref = escape_identifier(column_name)?;

    let alteration = match default {
        AlterColumnDefault::Drop => "DROP DEFAULT".to_string(),
        AlterColumnDefault::SetNull => "SET DEFAULT NULL".to_string(),
        AlterColumnDefault::Literal(pre_serialized) => {
            format!("SET DEFAULT {pre_serialized}")
        }
    };

    Ok(format!(
        "ALTER TABLE {table_ref} ALTER COLUMN {column_ref} {alteration}"
    ))
}

pub async fn alter_column_default(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    column_name: &str,
    default: AlterColumnDefault,
) -> Result<(), DdlError> {
    let sql = alter_column_default_sql(schema_name, table_name, column_name, default)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

fn backfill_update_sql(
    schema_name: &str,
    table_name: &str,
    column_name: &str,
    backfill_value: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "UPDATE {} SET {} = {} WHERE {} IS NULL",
        qualified_table(schema_name, table_name)?,
        escape_identifier(column_name)?,
        backfill_value,
        escape_identifier(column_name)?
    ))
}

pub fn set_column_nullable_sql(
    schema_name: &str,
    table_name: &str,
    column_name: &str,
    is_nullable: bool,
) -> Result<String, DdlError> {
    let alteration = if is_nullable {
        "DROP NOT NULL"
    } else {
        "SET NOT NULL"
    };

    Ok(format!(
        "ALTER TABLE {} ALTER COLUMN {} {alteration}",
        qualified_table(schema_name, table_name)?,
        escape_identifier(column_name)?
    ))
}

pub async fn alter_column_nullable(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    column_name: &str,
    is_nullable: bool,
    backfill_value: Option<&str>,
) -> Result<(), DdlError> {
    if let Some(backfill_value) = backfill_value {
        if !is_nullable && backfill_value != "NULL" {
            let sql = backfill_update_sql(schema_name, table_name, column_name, backfill_value)?;
            sqlx::query(&sql).execute(&mut *connection).await?;
        }
    }

    let sql = set_column_nullable_sql(schema_name, table_name, column_name, is_nullable)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

fn qualified_table(schema_name: &str, table_name: &str) -> Result<String, DdlError> {
    Ok(format!(
        "{}.{}",
        escape_identifier(schema_name)?,
        escape_identifier(table_name)?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_column() -> WorkspaceSchemaColumnDefinition {
        WorkspaceSchemaColumnDefinition {
            name: "name".to_string(),
            column_type: "text".to_string(),
            is_nullable: Some(false),
            default: Some("'unknown'::text".to_string()),
            is_primary: false,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        }
    }

    #[test]
    fn add_columns_sql_matches_the_upstream_manager() {
        assert_eq!(
            add_columns_sql("workspace_x", "person", &[text_column()], false).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ADD COLUMN \"name\" text NOT NULL DEFAULT 'unknown'::text"
        );
        assert_eq!(
            add_columns_sql("workspace_x", "person", &[text_column()], true).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ADD COLUMN IF NOT EXISTS \"name\" text NOT NULL DEFAULT 'unknown'::text"
        );
        assert!(add_columns_sql("workspace_x", "person", &[], false)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn add_columns_sql_joins_multiple_definitions_with_a_comma() {
        let second = WorkspaceSchemaColumnDefinition {
            name: "age".to_string(),
            column_type: "float".to_string(),
            is_nullable: Some(true),
            default: Some("NULL".to_string()),
            is_primary: false,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        };

        assert_eq!(
            add_columns_sql("workspace_x", "person", &[text_column(), second], false).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ADD COLUMN \"name\" text NOT NULL DEFAULT 'unknown'::text, ADD COLUMN \"age\" float DEFAULT NULL"
        );
    }

    #[test]
    fn drop_columns_sql_matches_the_upstream_manager() {
        assert_eq!(
            drop_columns_sql("workspace_x", "person", &["name", "age"], false).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" DROP COLUMN IF EXISTS \"name\", DROP COLUMN IF EXISTS \"age\""
        );
        assert_eq!(
            drop_columns_sql("workspace_x", "person", &["name"], true).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" DROP COLUMN IF EXISTS \"name\" CASCADE"
        );
        assert!(drop_columns_sql("workspace_x", "person", &[], false)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn rename_column_sql_matches_the_upstream_manager() {
        assert_eq!(
            rename_column_sql("workspace_x", "person", "name", "fullName").unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" RENAME COLUMN \"name\" TO \"fullName\""
        );
    }

    #[test]
    fn alter_column_default_sql_covers_drop_null_and_literal() {
        assert_eq!(
            alter_column_default_sql("workspace_x", "person", "name", AlterColumnDefault::Drop)
                .unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ALTER COLUMN \"name\" DROP DEFAULT"
        );
        assert_eq!(
            alter_column_default_sql("workspace_x", "person", "name", AlterColumnDefault::SetNull)
                .unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ALTER COLUMN \"name\" SET DEFAULT NULL"
        );
        assert_eq!(
            alter_column_default_sql(
                "workspace_x",
                "person",
                "name",
                AlterColumnDefault::Literal("'unknown'::text".to_string())
            )
            .unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ALTER COLUMN \"name\" SET DEFAULT 'unknown'::text"
        );
    }

    #[test]
    fn set_column_nullable_sql_switches_between_set_and_drop_not_null() {
        assert_eq!(
            set_column_nullable_sql("workspace_x", "person", "name", false).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ALTER COLUMN \"name\" SET NOT NULL"
        );
        assert_eq!(
            set_column_nullable_sql("workspace_x", "person", "name", true).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ALTER COLUMN \"name\" DROP NOT NULL"
        );
    }

    #[tokio::test]
    async fn column_nullability_backfill_round_trips_when_the_database_is_available() {
        let Some(mut connection) = super::super::test_support::connect().await else {
            return;
        };

        let schema_name = super::super::test_support::unique_schema_name("wm_column");
        super::super::test_support::create_schema(&mut connection, &schema_name).await;

        crate::workspace_migrations::ddl::table::create_table(
            &mut connection,
            &schema_name,
            "person",
            &[WorkspaceSchemaColumnDefinition {
                name: "name".to_string(),
                column_type: "text".to_string(),
                is_nullable: Some(true),
                default: None,
                is_primary: false,
                is_array: false,
                as_expression: None,
                generated_type: None,
                is_generated_always_as_identity: false,
            }],
        )
        .await
        .expect("create table");

        sqlx::query(&format!(
            "INSERT INTO \"{schema_name}\".\"person\"(\"name\") VALUES (NULL), ('filled')"
        ))
        .execute(&mut connection)
        .await
        .expect("seed rows");

        alter_column_nullable(
            &mut connection,
            &schema_name,
            "person",
            "name",
            false,
            Some("'backfilled'::text"),
        )
        .await
        .expect("set not null with backfill");

        let nulls: (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM \"{schema_name}\".\"person\" WHERE \"name\" IS NULL"
        ))
        .fetch_one(&mut connection)
        .await
        .expect("count nulls");

        assert_eq!(nulls.0, 0);

        alter_column_default(
            &mut connection,
            &schema_name,
            "person",
            "name",
            AlterColumnDefault::Literal("'new'::text".to_string()),
        )
        .await
        .expect("set default");

        sqlx::query(&format!(
            "INSERT INTO \"{schema_name}\".\"person\" DEFAULT VALUES"
        ))
        .execute(&mut connection)
        .await
        .expect("insert default row");

        let defaults: (i64,) = sqlx::query_as(&format!(
            "SELECT count(*) FROM \"{schema_name}\".\"person\" WHERE \"name\" = 'new'"
        ))
        .fetch_one(&mut connection)
        .await
        .expect("count defaults");

        assert_eq!(defaults.0, 1);

        super::super::test_support::drop_schema(&mut connection, &schema_name).await;
    }
}
