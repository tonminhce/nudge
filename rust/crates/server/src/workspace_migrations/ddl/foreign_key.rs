#![allow(dead_code)]

use sqlx::PgConnection;

use crate::metadata::ids::foreign_key_name;

use super::error::DdlError;
use super::escape::escape_identifier;

const ALLOWED_FK_ACTIONS: [&str; 5] = [
    "CASCADE",
    "SET NULL",
    "RESTRICT",
    "NO ACTION",
    "SET DEFAULT",
];

#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceSchemaForeignKeyDefinition {
    pub table_name: String,
    pub column_name: String,
    pub referenced_table_name: String,
    pub referenced_column_name: String,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

pub fn foreign_key_constraint_name(foreign_key: &WorkspaceSchemaForeignKeyDefinition) -> String {
    foreign_key_name(&foreign_key.table_name, &foreign_key.column_name)
}

pub fn create_foreign_key_sql(
    schema_name: &str,
    foreign_key: &WorkspaceSchemaForeignKeyDefinition,
    is_not_valid: bool,
) -> Result<String, DdlError> {
    let constraint_name = foreign_key_constraint_name(foreign_key);

    let mut sql = format!(
        "ALTER TABLE {}.{} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}.{} ({})",
        escape_identifier(schema_name)?,
        escape_identifier(&foreign_key.table_name)?,
        escape_identifier(&constraint_name)?,
        escape_identifier(&foreign_key.column_name)?,
        escape_identifier(schema_name)?,
        escape_identifier(&foreign_key.referenced_table_name)?,
        escape_identifier(&foreign_key.referenced_column_name)?
    );

    if let Some(on_delete) = &foreign_key.on_delete {
        if !ALLOWED_FK_ACTIONS.contains(&on_delete.as_str()) {
            return Err(DdlError::unsupported_on_delete_action(on_delete));
        }

        sql += &format!(" ON DELETE {on_delete}");
    }

    if let Some(on_update) = &foreign_key.on_update {
        if !ALLOWED_FK_ACTIONS.contains(&on_update.as_str()) {
            return Err(DdlError::unsupported_on_update_action(on_update));
        }

        sql += &format!(" ON UPDATE {on_update}");
    }

    if is_not_valid {
        sql += " NOT VALID";
    }

    Ok(sql)
}

pub async fn create_foreign_key(
    connection: &mut PgConnection,
    schema_name: &str,
    foreign_key: &WorkspaceSchemaForeignKeyDefinition,
    is_not_valid: bool,
) -> Result<(), DdlError> {
    let sql = create_foreign_key_sql(schema_name, foreign_key, is_not_valid)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn validate_foreign_key_sql(
    schema_name: &str,
    foreign_key: &WorkspaceSchemaForeignKeyDefinition,
) -> Result<String, DdlError> {
    let constraint_name = foreign_key_constraint_name(foreign_key);

    Ok(format!(
        "ALTER TABLE {}.{} VALIDATE CONSTRAINT {}",
        escape_identifier(schema_name)?,
        escape_identifier(&foreign_key.table_name)?,
        escape_identifier(&constraint_name)?
    ))
}

pub async fn validate_foreign_key(
    connection: &mut PgConnection,
    schema_name: &str,
    foreign_key: &WorkspaceSchemaForeignKeyDefinition,
) -> Result<(), DdlError> {
    let sql = validate_foreign_key_sql(schema_name, foreign_key)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn drop_foreign_key_sql(
    schema_name: &str,
    table_name: &str,
    foreign_key_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TABLE {}.{} DROP CONSTRAINT IF EXISTS {}",
        escape_identifier(schema_name)?,
        escape_identifier(table_name)?,
        escape_identifier(foreign_key_name)?
    ))
}

pub async fn drop_foreign_key(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    foreign_key_name: &str,
) -> Result<(), DdlError> {
    let sql = drop_foreign_key_sql(schema_name, table_name, foreign_key_name)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn set_foreign_key_not_deferrable_sql(
    schema_name: &str,
    table_name: &str,
    foreign_key_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TABLE {}.{} ALTER CONSTRAINT {} NOT DEFERRABLE",
        escape_identifier(schema_name)?,
        escape_identifier(table_name)?,
        escape_identifier(foreign_key_name)?
    ))
}

pub async fn set_foreign_key_not_deferrable(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    foreign_key_name: &str,
) -> Result<(), DdlError> {
    let sql = set_foreign_key_not_deferrable_sql(schema_name, table_name, foreign_key_name)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub fn set_foreign_key_deferrable_sql(
    schema_name: &str,
    table_name: &str,
    foreign_key_name: &str,
) -> Result<String, DdlError> {
    Ok(format!(
        "ALTER TABLE {}.{} ALTER CONSTRAINT {} DEFERRABLE",
        escape_identifier(schema_name)?,
        escape_identifier(table_name)?,
        escape_identifier(foreign_key_name)?
    ))
}

pub async fn set_foreign_key_deferrable(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    foreign_key_name: &str,
) -> Result<(), DdlError> {
    let sql = set_foreign_key_deferrable_sql(schema_name, table_name, foreign_key_name)?;

    sqlx::query(&sql).execute(connection).await?;

    Ok(())
}

pub async fn get_foreign_key_name(
    connection: &mut PgConnection,
    schema_name: &str,
    table_name: &str,
    column_name: &str,
) -> Result<Option<String>, DdlError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "
      SELECT
        tc.constraint_name AS constraint_name
      FROM
        information_schema.table_constraints AS tc
      JOIN
        information_schema.key_column_usage AS kcu
        ON tc.constraint_name = kcu.constraint_name
        AND tc.table_schema = kcu.table_schema
      WHERE
        tc.constraint_type = 'FOREIGN KEY'
        AND tc.table_schema = $1
        AND tc.table_name = $2
        AND kcu.column_name = $3
    ",
    )
    .bind(schema_name)
    .bind(table_name)
    .bind(column_name)
    .fetch_all(connection)
    .await?;

    Ok(rows
        .into_iter()
        .next()
        .map(|(constraint_name,)| constraint_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person_company_fk() -> WorkspaceSchemaForeignKeyDefinition {
        WorkspaceSchemaForeignKeyDefinition {
            table_name: "person".to_string(),
            column_name: "companyId".to_string(),
            referenced_table_name: "company".to_string(),
            referenced_column_name: "id".to_string(),
            on_delete: Some("SET NULL".to_string()),
            on_update: None,
        }
    }

    #[test]
    fn create_foreign_key_sql_is_byte_equal_to_the_upstream_manager() {
        assert_eq!(
            create_foreign_key_sql("workspace_x", &person_company_fk(), false).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ADD CONSTRAINT \"FK_ee066ddacfce46c9a7cb90edd1a\" FOREIGN KEY (\"companyId\") REFERENCES \"workspace_x\".\"company\" (\"id\") ON DELETE SET NULL"
        );
    }

    #[test]
    fn create_foreign_key_sql_appends_not_valid_and_on_update() {
        let mut foreign_key = person_company_fk();
        foreign_key.on_update = Some("NO ACTION".to_string());

        assert_eq!(
            create_foreign_key_sql("workspace_x", &foreign_key, true).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" ADD CONSTRAINT \"FK_ee066ddacfce46c9a7cb90edd1a\" FOREIGN KEY (\"companyId\") REFERENCES \"workspace_x\".\"company\" (\"id\") ON DELETE SET NULL ON UPDATE NO ACTION NOT VALID"
        );
    }

    #[test]
    fn create_foreign_key_sql_rejects_unknown_actions() {
        let mut foreign_key = person_company_fk();
        foreign_key.on_delete = Some("DELETE ALL".to_string());

        let error = create_foreign_key_sql("workspace_x", &foreign_key, false).unwrap_err();
        assert_eq!(error.code(), "UNSUPPORTED_ON_DELETE_ACTION");

        let mut foreign_key = person_company_fk();
        foreign_key.on_update = Some("UPGRADE".to_string());

        let error = create_foreign_key_sql("workspace_x", &foreign_key, false).unwrap_err();
        assert_eq!(error.code(), "UNSUPPORTED_ON_UPDATE_ACTION");
    }

    #[test]
    fn validate_and_drop_and_deferrable_sql_match_the_upstream_manager() {
        assert_eq!(
            validate_foreign_key_sql("workspace_x", &person_company_fk()).unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" VALIDATE CONSTRAINT \"FK_ee066ddacfce46c9a7cb90edd1a\""
        );
        assert_eq!(
            drop_foreign_key_sql("workspace_x", "person", "FK_ee066ddacfce46c9a7cb90edd1a").unwrap(),
            "ALTER TABLE \"workspace_x\".\"person\" DROP CONSTRAINT IF EXISTS \"FK_ee066ddacfce46c9a7cb90edd1a\""
        );
        assert_eq!(
            set_foreign_key_deferrable_sql("workspace_x", "noteTarget", "FK_52050fd9cf6b7ba9982ef9d867a")
                .unwrap(),
            "ALTER TABLE \"workspace_x\".\"noteTarget\" ALTER CONSTRAINT \"FK_52050fd9cf6b7ba9982ef9d867a\" DEFERRABLE"
        );
        assert_eq!(
            set_foreign_key_not_deferrable_sql("workspace_x", "noteTarget", "FK_52050fd9cf6b7ba9982ef9d867a")
                .unwrap(),
            "ALTER TABLE \"workspace_x\".\"noteTarget\" ALTER CONSTRAINT \"FK_52050fd9cf6b7ba9982ef9d867a\" NOT DEFERRABLE"
        );
    }

    #[test]
    fn constraint_names_come_from_the_typeorm_naming_strategy() {
        assert_eq!(
            foreign_key_constraint_name(&person_company_fk()),
            "FK_ee066ddacfce46c9a7cb90edd1a"
        );
    }

    #[tokio::test]
    async fn foreign_key_lifecycle_round_trips_when_the_database_is_available() {
        let Some(mut connection) = super::super::test_support::connect().await else {
            return;
        };

        let schema_name = super::super::test_support::unique_schema_name("wm_fk");
        super::super::test_support::create_schema(&mut connection, &schema_name).await;

        crate::workspace_migrations::ddl::table::create_table(
            &mut connection,
            &schema_name,
            "company",
            &[],
        )
        .await
        .expect("create company table");

        crate::workspace_migrations::ddl::table::create_table(
            &mut connection,
            &schema_name,
            "person",
            &[],
        )
        .await
        .expect("create person table");

        crate::workspace_migrations::ddl::column::add_columns(
            &mut connection,
            &schema_name,
            "person",
            &[
                crate::workspace_migrations::column_defs::WorkspaceSchemaColumnDefinition {
                    name: "companyId".to_string(),
                    column_type: "uuid".to_string(),
                    is_nullable: Some(true),
                    default: None,
                    is_primary: false,
                    is_array: false,
                    as_expression: None,
                    generated_type: None,
                    is_generated_always_as_identity: false,
                },
            ],
            false,
        )
        .await
        .expect("add join column");

        let mut foreign_key = person_company_fk();
        foreign_key.on_delete = Some("CASCADE".to_string());

        create_foreign_key(&mut connection, &schema_name, &foreign_key, true)
            .await
            .expect("create not valid foreign key");

        validate_foreign_key(&mut connection, &schema_name, &foreign_key)
            .await
            .expect("validate foreign key");

        let stored_name =
            get_foreign_key_name(&mut connection, &schema_name, "person", "companyId")
                .await
                .expect("get foreign key name");

        assert_eq!(
            stored_name,
            Some("FK_ee066ddacfce46c9a7cb90edd1a".to_string())
        );

        set_foreign_key_deferrable(
            &mut connection,
            &schema_name,
            "person",
            "FK_ee066ddacfce46c9a7cb90edd1a",
        )
        .await
        .expect("set deferrable");

        drop_foreign_key(
            &mut connection,
            &schema_name,
            "person",
            "FK_ee066ddacfce46c9a7cb90edd1a",
        )
        .await
        .expect("drop foreign key");

        let removed = get_foreign_key_name(&mut connection, &schema_name, "person", "companyId")
            .await
            .expect("get removed foreign key name");

        assert_eq!(removed, None);

        super::super::test_support::drop_schema(&mut connection, &schema_name).await;
    }
}
