#![allow(dead_code)]

use sqlx::PgConnection;

use super::error::DdlError;
use super::escape::escape_identifier;

const ALLOWED_INDEX_TYPES: [&str; 6] = ["BTREE", "HASH", "GIST", "SPGIST", "GIN", "BRIN"];

const ALLOWED_INDEX_WHERE_CLAUSES: [&str; 2] = ["\"deletedAt\" IS NULL", "\"_isPrivate\""];

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSchemaIndexDefinition {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub index_type: Option<String>,
    pub where_clause: Option<String>,
}

pub fn validate_and_return_index_where_clause(
    clause: Option<&str>,
) -> Result<Option<String>, DdlError> {
    match clause {
        None | Some("") => Ok(None),
        Some(clause) => {
            if ALLOWED_INDEX_WHERE_CLAUSES.contains(&clause) {
                Ok(Some(clause.to_string()))
            } else {
                Err(DdlError::unsupported_index_where_clause(clause))
            }
        }
    }
}

pub fn create_index_sql(
    schema_name: &str,
    table_name: &str,
    index: &WorkspaceSchemaIndexDefinition,
    concurrently: bool,
    is_transaction_active: bool,
) -> Result<String, DdlError> {
    if concurrently && is_transaction_active {
        return Err(DdlError::concurrent_index_creation_in_transaction());
    }

    let mut quoted_columns = Vec::with_capacity(index.columns.len());

    for column in &index.columns {
        quoted_columns.push(escape_identifier(column)?);
    }

    let is_unique = if index.is_unique { "UNIQUE" } else { "" };

    let mut index_type = String::new();

    if let Some(raw_index_type) = &index.index_type {
        if raw_index_type != "BTREE" {
            if !ALLOWED_INDEX_TYPES.contains(&raw_index_type.as_str()) {
                return Err(DdlError::unsupported_index_type(raw_index_type));
            }

            index_type = format!("USING {raw_index_type}");
        }
    }

    let validated_where_clause =
        validate_and_return_index_where_clause(index.where_clause.as_deref())?;
    let where_clause = validated_where_clause
        .map(|clause| format!("WHERE {clause}"))
        .unwrap_or_default();

    let concurrency = if concurrently { "CONCURRENTLY" } else { "" };

    let parts: Vec<String> = [
        Some("CREATE".to_string()),
        (!is_unique.is_empty()).then(|| is_unique.to_string()),
        Some("INDEX".to_string()),
        (!concurrency.is_empty()).then(|| concurrency.to_string()),
        Some("IF NOT EXISTS".to_string()),
        Some(escape_identifier(&index.name)?),
        Some("ON".to_string()),
        Some(format!(
            "{}.{}",
            escape_identifier(schema_name)?,
            escape_identifier(table_name)?
        )),
        (!index_type.is_empty()).then_some(index_type),
        Some(format!("({})", quoted_columns.join(", "))),
        (!where_clause.is_empty()).then_some(where_clause),
    ]
    .into_iter()
    .flatten()
    .collect();

    Ok(parts.join(" ").trim().to_string())
}

pub async fn create_index(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    index: &WorkspaceSchemaIndexDefinition,
) -> Result<(), DdlError> {
    let sql = create_index_sql(schema_name, table_name, index, false, true)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub async fn create_index_concurrently(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    index: &WorkspaceSchemaIndexDefinition,
) -> Result<(), DdlError> {
    let sql = create_index_sql(schema_name, table_name, index, true, false)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn drop_index_sql(
    schema_name: &str,
    index_name: &str,
    concurrently: bool,
) -> Result<String, DdlError> {
    let concurrency = if concurrently { "CONCURRENTLY " } else { "" };

    Ok(format!(
        "DROP INDEX {concurrency}IF EXISTS {}.{}",
        escape_identifier(schema_name)?,
        escape_identifier(index_name)?
    ))
}

pub async fn drop_index(
    connection: &mut PgConnection,
    schema_name: &str,
    index_name: &str,
    concurrently: bool,
) -> Result<(), DdlError> {
    let sql = drop_index_sql(schema_name, index_name, concurrently)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn rename_index_without_rebuild_sql(
    schema_name: &str,
    from_index_name: &str,
    to_index_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER INDEX {}.{} RENAME TO {}",
        escape_identifier(schema_name)?,
        escape_identifier(from_index_name)?,
        escape_identifier(to_index_name)?
    ))
}

pub async fn rename_index_without_rebuild(
    connection: &mut PgConnection,
    schema_name: &str,
    from_index_name: &str,
    to_index_name: &str,
) -> Result<(), DdlError> {
    let sql = rename_index_without_rebuild_sql(schema_name, from_index_name, to_index_name)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::workspace_migrations::column_defs::WorkspaceSchemaColumnDefinition;

    fn partial_unique_index() -> WorkspaceSchemaIndexDefinition {
        WorkspaceSchemaIndexDefinition {
            name: "IDX_UNIQUE_f68c4b0b86079af3bea4286d92e".to_string(),
            columns: vec!["emailsPrimaryEmail".to_string()],
            is_unique: true,
            index_type: None,
            where_clause: Some("\"deletedAt\" IS NULL".to_string()),
        }
    }

    fn person_columns() -> Vec<WorkspaceSchemaColumnDefinition> {
        vec![
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
            },
            WorkspaceSchemaColumnDefinition {
                name: "emailsPrimaryEmail".to_string(),
                column_type: "text".to_string(),
                is_nullable: Some(true),
                default: None,
                is_primary: false,
                is_array: false,
                as_expression: None,
                generated_type: None,
                is_generated_always_as_identity: false,
            },
            WorkspaceSchemaColumnDefinition {
                name: "deletedAt".to_string(),
                column_type: "timestamptz".to_string(),
                is_nullable: Some(true),
                default: None,
                is_primary: false,
                is_array: false,
                as_expression: None,
                generated_type: None,
                is_generated_always_as_identity: false,
            },
        ]
    }

    #[test]
    fn create_index_sql_is_byte_equal_to_the_upstream_manager() {
        assert_eq!(
            create_index_sql("workspace_x", "person", &partial_unique_index(), false, false).unwrap(),
            "CREATE UNIQUE INDEX IF NOT EXISTS \"IDX_UNIQUE_f68c4b0b86079af3bea4286d92e\" ON \"workspace_x\".\"person\" (\"emailsPrimaryEmail\") WHERE \"deletedAt\" IS NULL"
        );
    }

    #[test]
    fn create_index_sql_emits_using_for_non_btree_types() {
        let gin_index = WorkspaceSchemaIndexDefinition {
            name: "IDX_gin".to_string(),
            columns: vec!["searchVector".to_string()],
            is_unique: false,
            index_type: Some("GIN".to_string()),
            where_clause: None,
        };

        assert_eq!(
            create_index_sql("workspace_x", "person", &gin_index, false, false).unwrap(),
            "CREATE INDEX IF NOT EXISTS \"IDX_gin\" ON \"workspace_x\".\"person\" USING GIN (\"searchVector\")"
        );
    }

    #[test]
    fn create_index_sql_rejects_unsupported_types_and_where_clauses() {
        let bad_type = WorkspaceSchemaIndexDefinition {
            name: "IDX_bad".to_string(),
            columns: vec!["id".to_string()],
            is_unique: false,
            index_type: Some("BOGUS".to_string()),
            where_clause: None,
        };

        let error = create_index_sql("workspace_x", "person", &bad_type, false, false).unwrap_err();
        assert_eq!(error.code(), "UNSUPPORTED_INDEX_TYPE");

        let bad_where = WorkspaceSchemaIndexDefinition {
            where_clause: Some("\"id\" > 0".to_string()),
            ..partial_unique_index()
        };

        let error =
            create_index_sql("workspace_x", "person", &bad_where, false, false).unwrap_err();
        assert_eq!(error.code(), "UNSUPPORTED_INDEX_WHERE_CLAUSE");
        assert!(error
            .to_string()
            .contains("Unsupported index WHERE clause: \""));
        assert!(error.to_string().contains("\"id\" > 0"));
    }

    #[test]
    fn concurrently_is_rejected_inside_transactions_and_allowed_outside() {
        let error = create_index_sql("workspace_x", "person", &partial_unique_index(), true, true)
            .unwrap_err();
        assert_eq!(error.code(), "CONCURRENT_INDEX_CREATION_IN_TRANSACTION");

        assert_eq!(
            create_index_sql("workspace_x", "person", &partial_unique_index(), true, false).unwrap(),
            "CREATE UNIQUE INDEX CONCURRENTLY IF NOT EXISTS \"IDX_UNIQUE_f68c4b0b86079af3bea4286d92e\" ON \"workspace_x\".\"person\" (\"emailsPrimaryEmail\") WHERE \"deletedAt\" IS NULL"
        );
    }

    #[test]
    fn validate_index_where_clause_allowlist() {
        assert_eq!(
            validate_and_return_index_where_clause(Some("\"deletedAt\" IS NULL")).unwrap(),
            Some("\"deletedAt\" IS NULL".to_string())
        );
        assert_eq!(
            validate_and_return_index_where_clause(Some("\"_isPrivate\"")).unwrap(),
            Some("\"_isPrivate\"".to_string())
        );
        assert_eq!(validate_and_return_index_where_clause(None).unwrap(), None);
        assert_eq!(
            validate_and_return_index_where_clause(Some("")).unwrap(),
            None
        );
    }

    #[test]
    fn drop_and_rename_index_sql_match_the_upstream_manager() {
        assert_eq!(
            drop_index_sql("workspace_x", "IDX_gin", false).unwrap(),
            "DROP INDEX IF EXISTS \"workspace_x\".\"IDX_gin\""
        );
        assert_eq!(
            drop_index_sql("workspace_x", "IDX_gin", true).unwrap(),
            "DROP INDEX CONCURRENTLY IF EXISTS \"workspace_x\".\"IDX_gin\""
        );
        assert_eq!(
            rename_index_without_rebuild_sql("workspace_x", "IDX_old", "IDX_new").unwrap(),
            "ALTER INDEX \"workspace_x\".\"IDX_old\" RENAME TO \"IDX_new\""
        );
    }

    #[tokio::test]
    async fn index_lifecycle_round_trips_when_the_database_is_available() {
        let Some(mut connection) = super::super::test_support::connect().await else {
            return;
        };

        let schema_name = super::super::test_support::unique_schema_name("wm_index");
        super::super::test_support::create_schema(&mut connection, &schema_name).await;

        crate::workspace_migrations::ddl::table::create_table(
            &mut connection,
            &schema_name,
            "person",
            &person_columns(),
        )
        .await
        .expect("create table");

        create_index(
            &mut connection,
            &schema_name,
            "person",
            &partial_unique_index(),
        )
        .await
        .expect("create index");

        let indexes: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM pg_indexes WHERE schemaname = $1 AND indexname = $2",
        )
        .bind(&schema_name)
        .bind("IDX_UNIQUE_f68c4b0b86079af3bea4286d92e")
        .fetch_one(&mut connection)
        .await
        .expect("count indexes");

        assert_eq!(indexes.0, 1);

        rename_index_without_rebuild(
            &mut connection,
            &schema_name,
            "IDX_UNIQUE_f68c4b0b86079af3bea4286d92e",
            "IDX_renamed",
        )
        .await
        .expect("rename index");

        drop_index(&mut connection, &schema_name, "IDX_renamed", false)
            .await
            .expect("drop index");

        let dropped: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM pg_indexes WHERE schemaname = $1 AND indexname = 'IDX_renamed'",
        )
        .bind(&schema_name)
        .fetch_one(&mut connection)
        .await
        .expect("count renamed index");

        assert_eq!(dropped.0, 0);

        super::super::test_support::drop_schema(&mut connection, &schema_name).await;
    }

    #[tokio::test]
    async fn concurrent_index_creation_outside_a_transaction_succeeds() {
        let Some(mut connection) = super::super::test_support::connect().await else {
            return;
        };

        let schema_name = super::super::test_support::unique_schema_name("wm_concurrent");
        super::super::test_support::create_schema(&mut connection, &schema_name).await;

        crate::workspace_migrations::ddl::table::create_table(
            &mut connection,
            &schema_name,
            "person",
            &person_columns(),
        )
        .await
        .expect("create table");

        create_index_concurrently(
            &mut connection,
            &schema_name,
            "person",
            &partial_unique_index(),
        )
        .await
        .expect("create index concurrently");

        super::super::test_support::drop_schema(&mut connection, &schema_name).await;
    }
}
