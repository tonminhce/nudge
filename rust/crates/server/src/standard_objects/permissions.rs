use sqlx::PgConnection;
use sqlx::PgPool;

use crate::workspace_migrations::ddl::escape::escape_identifier;

pub const ACCESS_GROUP_UNIVERSAL_IDENTIFIER: &str = "47f4ed44-4312-467f-b6f5-787f91037a1d";
pub const RECORD_SHARE_UNIVERSAL_IDENTIFIER: &str = "fecdb92c-d150-4544-81f8-f1158d18a7a2";

const PERMISSION_TABLE_NAMES: [&str; 2] = ["accessGroup", "recordShare"];

const RECORD_SHARE_STATEMENTS: [&str; 7] = [
    "CREATE TYPE {schema}.\"recordShare_principalType_enum\" AS ENUM ('EVERYONE', 'WORKSPACE_MEMBER', 'ROLE')",
    "CREATE TYPE {schema}.\"recordShare_rowCause_enum\" AS ENUM ('OWNER', 'MANUAL', 'RULE', 'APPLICATION')",
    "CREATE TYPE {schema}.\"recordShare_accessLevel_enum\" AS ENUM ('READ', 'READ_WRITE', 'FULL', 'NONE')",
    "CREATE TABLE IF NOT EXISTS {schema}.\"recordShare\" (\"id\" uuid PRIMARY KEY NOT NULL DEFAULT public.uuid_generate_v4(), \"createdAt\" timestamp with time zone NOT NULL DEFAULT now(), \"updatedAt\" timestamp with time zone NOT NULL DEFAULT now(), \"deletedAt\" timestamp with time zone, \"recordId\" uuid NOT NULL, \"objectMetadataId\" uuid NOT NULL, \"principalId\" uuid NOT NULL, \"principalType\" {schema}.\"recordShare_principalType_enum\" NOT NULL, \"accessLevel\" {schema}.\"recordShare_accessLevel_enum\" NOT NULL DEFAULT 'READ', \"rowCause\" {schema}.\"recordShare_rowCause_enum\" NOT NULL, \"sourceId\" uuid NOT NULL)",
    "CREATE UNIQUE INDEX IF NOT EXISTS \"IDX_UNIQUE_2a8112c19cab880dde4b9b122e0\" ON {schema}.\"recordShare\" USING btree (\"objectMetadataId\", \"recordId\", \"principalId\", \"rowCause\", \"sourceId\")",
    "CREATE INDEX IF NOT EXISTS \"IDX_4a277b3dad18715d2619fee7c5b\" ON {schema}.\"recordShare\" USING btree (\"principalId\")",
    "CREATE INDEX IF NOT EXISTS \"IDX_5519b21a3b491ab8b1ce594b119\" ON {schema}.\"recordShare\" USING btree (\"sourceId\")",
];

const ACCESS_GROUP_STATEMENTS: [&str; 7] = [
    "CREATE TYPE {schema}.\"accessGroup_type_enum\" AS ENUM ('ALL_RECORDS', 'RLS_RULE', 'RLS_MEMBER_VALUE', 'RLS_LITERAL_VALUE')",
    "CREATE TABLE IF NOT EXISTS {schema}.\"accessGroup\" (\"id\" uuid PRIMARY KEY NOT NULL DEFAULT public.uuid_generate_v4(), \"createdAt\" timestamp with time zone NOT NULL DEFAULT now(), \"updatedAt\" timestamp with time zone NOT NULL DEFAULT now(), \"deletedAt\" timestamp with time zone, \"objectMetadataId\" uuid, \"type\" {schema}.\"accessGroup_type_enum\" NOT NULL, \"ruleShapeHash\" character varying NOT NULL, \"unreadableFieldIds\" uuid[] NOT NULL DEFAULT '{}'::uuid[], \"memberFieldValues\" jsonb, \"value\" character varying, \"groupHash\" text NOT NULL, \"compactId\" integer GENERATED ALWAYS AS IDENTITY, CONSTRAINT \"CHK_ACCESS_GROUP_TYPE_COLUMNS\" CHECK ((\"type\" = ANY (ARRAY['ALL_RECORDS'::{schema}.\"accessGroup_type_enum\", 'RLS_RULE'::{schema}.\"accessGroup_type_enum\"]) AND \"memberFieldValues\" IS NULL AND \"value\" IS NULL) OR (\"type\" = 'RLS_MEMBER_VALUE'::{schema}.\"accessGroup_type_enum\" AND \"memberFieldValues\" IS NOT NULL AND \"value\" IS NULL) OR (\"type\" = 'RLS_LITERAL_VALUE'::{schema}.\"accessGroup_type_enum\" AND \"memberFieldValues\" IS NULL AND \"value\" IS NOT NULL)))",
    "CREATE UNIQUE INDEX IF NOT EXISTS \"IDX_UNIQUE_b784cebdb5d8238240a4b99b8e2\" ON {schema}.\"accessGroup\" USING btree (\"groupHash\")",
    "CREATE UNIQUE INDEX IF NOT EXISTS \"IDX_ACCESS_GROUP_COMPACT_ID_UNIQUE\" ON {schema}.\"accessGroup\" USING btree (\"compactId\")",
    "CREATE INDEX IF NOT EXISTS \"IDX_3794d47209882e7c43f4f786f33\" ON {schema}.\"accessGroup\" USING btree (\"objectMetadataId\")",
    "CREATE INDEX IF NOT EXISTS \"IDX_ACCESS_GROUP_RULE_SHAPE_MEMBER_FIELD_VALUES\" ON {schema}.\"accessGroup\" USING btree (\"ruleShapeHash\", \"memberFieldValues\") WHERE \"type\" = 'RLS_MEMBER_VALUE'",
    "CREATE INDEX IF NOT EXISTS \"IDX_ACCESS_GROUP_RULE_SHAPE_VALUE\" ON {schema}.\"accessGroup\" USING btree (\"ruleShapeHash\", \"value\") WHERE \"type\" = 'RLS_LITERAL_VALUE'",
];

pub fn permission_storage_universal_identifiers() -> &'static [&'static str] {
    &[
        ACCESS_GROUP_UNIVERSAL_IDENTIFIER,
        RECORD_SHARE_UNIVERSAL_IDENTIFIER,
    ]
}

pub fn is_permission_storage_object(universal_identifier: &str) -> bool {
    permission_storage_universal_identifiers().contains(&universal_identifier)
}

pub fn is_permission_storage_table(table_name: &str) -> bool {
    PERMISSION_TABLE_NAMES.contains(&table_name)
}

fn render(statement: &str, schema_name: &str) -> Result<String, PermissionTableError> {
    let quoted = escape_identifier(schema_name)
        .map_err(|error| PermissionTableError::new(error.to_string()))?;

    Ok(statement.replace("{schema}", &quoted))
}

#[derive(Debug)]
pub struct PermissionTableError {
    pub message: String,
}

impl PermissionTableError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<sqlx::Error> for PermissionTableError {
    fn from(error: sqlx::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl std::fmt::Display for PermissionTableError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "PERMISSION_TABLE_FAILED: {}", self.message)
    }
}

impl std::error::Error for PermissionTableError {}

async fn enum_exists(
    connection: &mut PgConnection,
    schema_name: &str,
    name: &str,
) -> Result<bool, sqlx::Error> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace \
         WHERE n.nspname = $1 AND t.typname = $2)",
    )
    .bind(schema_name)
    .bind(name)
    .fetch_one(connection)
    .await?;

    Ok(row.0)
}

pub async fn create_permission_tables(
    connection: &mut PgConnection,
    schema_name: &str,
) -> Result<(), PermissionTableError> {
    for (statement, enum_name) in RECORD_SHARE_STATEMENTS
        .iter()
        .take(3)
        .zip([
            "recordShare_principalType_enum",
            "recordShare_rowCause_enum",
            "recordShare_accessLevel_enum",
        ])
        .chain(
            ACCESS_GROUP_STATEMENTS
                .iter()
                .take(1)
                .zip(["accessGroup_type_enum"]),
        )
    {
        if !enum_exists(&mut *connection, schema_name, enum_name).await? {
            sqlx::query(&render(statement, schema_name)?)
                .execute(&mut *connection)
                .await?;
        }
    }

    for statement in RECORD_SHARE_STATEMENTS.iter().skip(3) {
        sqlx::query(&render(statement, schema_name)?)
            .execute(&mut *connection)
            .await?;
    }

    for statement in ACCESS_GROUP_STATEMENTS.iter().skip(1) {
        sqlx::query(&render(statement, schema_name)?)
            .execute(&mut *connection)
            .await?;
    }

    Ok(())
}

pub async fn drop_permission_tables(pool: &PgPool, schema_name: &str) -> Result<(), sqlx::Error> {
    for table in PERMISSION_TABLE_NAMES {
        let schema = escape_identifier(schema_name)
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
        let table =
            escape_identifier(table).map_err(|error| sqlx::Error::Protocol(error.to_string()))?;

        sqlx::query(&format!("DROP TABLE IF EXISTS {schema}.{table} CASCADE"))
            .execute(pool)
            .await?;
    }

    Ok(())
}

pub async fn permission_table_index_names(
    pool: &PgPool,
    schema_name: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT indexname FROM pg_indexes WHERE schemaname = $1 AND tablename = ANY($2) \
         ORDER BY indexname",
    )
    .bind(schema_name)
    .bind(&PERMISSION_TABLE_NAMES[..])
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(name,)| name).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn permission_storage_identifiers_are_stable() {
        assert!(is_permission_storage_object(
            ACCESS_GROUP_UNIVERSAL_IDENTIFIER
        ));
        assert!(is_permission_storage_object(
            RECORD_SHARE_UNIVERSAL_IDENTIFIER
        ));
        assert!(!is_permission_storage_object(
            "20202020-e674-48e5-a542-72570eee7213"
        ));
    }

    #[test]
    fn statements_reference_the_quoted_schema() {
        let rendered = render(RECORD_SHARE_STATEMENTS[3], "workspace_abc123").expect("rendering");

        assert!(rendered.contains("\"workspace_abc123\".\"recordShare\""));
        assert!(rendered.starts_with("CREATE TABLE IF NOT EXISTS"));
    }

    #[tokio::test]
    async fn tables_round_trip_against_postgres_when_the_database_is_available() {
        let Ok(url) = std::env::var("NUDGE_TEST_DATABASE_URL") else {
            return;
        };

        let pool = sqlx::PgPool::connect(&url).await.expect("connecting");
        let schema = format!("perm_{}", &Uuid::new_v4().simple().to_string()[..12]);

        sqlx::query(&format!("CREATE SCHEMA \"{schema}\""))
            .execute(&pool)
            .await
            .expect("creating the schema");

        let mut connection = pool.acquire().await.expect("acquiring");

        crate::workspace_migrations::ddl::test_support::setup_upstream_extensions(&mut connection)
            .await;

        create_permission_tables(&mut connection, &schema)
            .await
            .expect("creating permission tables");

        let count: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = $1",
        )
        .bind(&schema)
        .fetch_one(&pool)
        .await
        .expect("counting tables");

        assert_eq!(count.0, 2);

        let indexes = permission_table_index_names(&pool, &schema)
            .await
            .expect("listing indexes");

        assert_eq!(indexes.len(), 10);
        assert!(indexes.contains(&"IDX_ACCESS_GROUP_COMPACT_ID_UNIQUE".to_string()));
        assert!(indexes.contains(&"IDX_UNIQUE_2a8112c19cab880dde4b9b122e0".to_string()));

        create_permission_tables(&mut connection, &schema)
            .await
            .expect("re-running permission table creation is idempotent");

        drop(connection);

        sqlx::query(&format!("DROP SCHEMA \"{schema}\" CASCADE"))
            .execute(&pool)
            .await
            .expect("dropping the schema");
    }
}
