#![allow(dead_code)]

use sqlx::PgConnection;

use crate::workspace_migrations::column_defs::build_sql_column_definition;
use crate::workspace_migrations::column_defs::WorkspaceSchemaColumnDefinition;

use super::error::DdlError;
use super::escape::escape_identifier;

const BARE_TABLE_COLUMN_DEFINITIONS: [&str; 1] =
    ["\"id\" uuid PRIMARY KEY DEFAULT gen_random_uuid()"];

pub fn create_table_sql(
    schema_name: &str,
    table_name: &str,
    column_definitions: &[WorkspaceSchemaColumnDefinition],
) -> Result<String, DdlError> {
    let mut sql_column_definitions = Vec::with_capacity(column_definitions.len());

    for definition in column_definitions {
        sql_column_definitions.push(build_sql_column_definition(definition)?);
    }

    if sql_column_definitions.is_empty() {
        sql_column_definitions.extend(
            BARE_TABLE_COLUMN_DEFINITIONS
                .iter()
                .map(|part| (*part).to_string()),
        );
    }

    Ok(format!(
        "CREATE TABLE IF NOT EXISTS {} ({})",
        qualified_table(schema_name, table_name)?,
        sql_column_definitions.join(", ")
    ))
}

pub async fn create_table(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    column_definitions: &[WorkspaceSchemaColumnDefinition],
) -> Result<(), DdlError> {
    let sql = create_table_sql(schema_name, table_name, column_definitions)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn drop_table_sql(
    schema_name: &str,
    table_name: &str,
    cascade: bool,
) -> Result<String, DdlError> {
    let cascade_clause = if cascade { " CASCADE" } else { "" };

    Ok(format!(
        "DROP TABLE IF EXISTS {}{cascade_clause}",
        qualified_table(schema_name, table_name)?
    ))
}

pub async fn drop_table(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    cascade: bool,
) -> Result<(), DdlError> {
    let sql = drop_table_sql(schema_name, table_name, cascade)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn rename_table_sql(
    schema_name: &str,
    old_table_name: &str,
    new_table_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TABLE {} RENAME TO {}",
        qualified_table(schema_name, old_table_name)?,
        escape_identifier(new_table_name)?
    ))
}

pub async fn rename_table(
    connection: &mut PgConnection,
    schema_name: &str,
    old_table_name: &str,
    new_table_name: &str,
) -> Result<(), DdlError> {
    let sql = rename_table_sql(schema_name, old_table_name, new_table_name)?;

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

    fn id_column() -> WorkspaceSchemaColumnDefinition {
        WorkspaceSchemaColumnDefinition {
            name: "id".to_string(),
            column_type: "uuid".to_string(),
            is_nullable: Some(false),
            default: Some("public.uuid_generate_v4()".to_string()),
            is_primary: true,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        }
    }

    fn name_column() -> WorkspaceSchemaColumnDefinition {
        WorkspaceSchemaColumnDefinition {
            name: "name".to_string(),
            column_type: "text".to_string(),
            is_nullable: Some(true),
            default: Some("NULL".to_string()),
            is_primary: false,
            is_array: false,
            as_expression: None,
            generated_type: None,
            is_generated_always_as_identity: false,
        }
    }

    #[test]
    fn create_table_sql_is_byte_equal_to_the_upstream_manager() {
        let sql = create_table_sql("workspace_x", "person", &[id_column(), name_column()]).unwrap();

        assert_eq!(
            sql,
            "CREATE TABLE IF NOT EXISTS \"workspace_x\".\"person\" (\"id\" uuid PRIMARY KEY NOT NULL DEFAULT public.uuid_generate_v4(), \"name\" text DEFAULT NULL)"
        );
    }

    #[test]
    fn create_table_sql_falls_back_to_the_bare_id_column() {
        let sql = create_table_sql("workspace_x", "empty", &[]).unwrap();

        assert_eq!(
            sql,
            "CREATE TABLE IF NOT EXISTS \"workspace_x\".\"empty\" (\"id\" uuid PRIMARY KEY DEFAULT gen_random_uuid())"
        );
    }

    #[test]
    fn drop_table_sql_appends_cascade_only_when_asked() {
        assert_eq!(
            drop_table_sql("workspace_x", "person", false).unwrap(),
            "DROP TABLE IF EXISTS \"workspace_x\".\"person\""
        );
        assert_eq!(
            drop_table_sql("workspace_x", "person", true).unwrap(),
            "DROP TABLE IF EXISTS \"workspace_x\".\"person\" CASCADE"
        );
    }

    #[test]
    fn rename_table_sql_quotes_schema_table_and_new_name() {
        assert_eq!(
            rename_table_sql("workspace_x", "person", "people").unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" RENAME TO \"people\""
        );
    }

    #[tokio::test]
    async fn table_round_trip_against_postgres_when_the_database_is_available() {
        let Some(mut connection) = super::super::test_support::connect().await else {
            return;
        };

        let schema_name = super::super::test_support::unique_schema_name("wm_table");
        super::super::test_support::create_schema(&mut connection, &schema_name).await;
        super::super::test_support::setup_upstream_extensions(&mut connection).await;

        create_table(
            &mut connection,
            &schema_name,
            "person",
            &[id_column(), name_column()],
        )
        .await
        .expect("create table");

        let created: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = $1 AND table_name = 'person'",
        )
        .bind(&schema_name)
        .fetch_one(&mut connection)
        .await
        .expect("table existence query");

        assert_eq!(created.0, 1);

        rename_table(&mut connection, &schema_name, "person", "people")
            .await
            .expect("rename table");

        drop_table(&mut connection, &schema_name, "people", true)
            .await
            .expect("drop table");

        let dropped: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = $1 AND table_name = 'people'",
        )
        .bind(&schema_name)
        .fetch_one(&mut connection)
        .await
        .expect("dropped table query");

        assert_eq!(dropped.0, 0);

        super::super::test_support::drop_schema(&mut connection, &schema_name).await;
    }
}
