#![allow(dead_code)]

pub mod column;
pub mod enum_types;
pub mod error;
pub mod escape;
pub mod foreign_key;
pub mod index;
pub mod table;

#[cfg(test)]
pub(crate) mod test_support {
    use sqlx::{Connection, PgConnection};

    pub async fn connect() -> Option<PgConnection> {
        let url = std::env::var("NUDGE_TEST_DATABASE_URL").ok()?;
        PgConnection::connect(&url).await.ok()
    }

    pub async fn migrate_core(url: &str) -> std::result::Result<(), String> {
        let mut connection = PgConnection::connect(url)
            .await
            .map_err(|error| error.to_string())?;

        sqlx::query("SET search_path TO public")
            .execute(&mut connection)
            .await
            .map_err(|error| error.to_string())?;

        sqlx::query("SELECT pg_advisory_lock(hashtext('nudge-core-migrations'))")
            .execute(&mut connection)
            .await
            .map_err(|error| error.to_string())?;

        let result = sqlx::migrate!("./migrations/core")
            .run(&mut connection)
            .await;

        sqlx::query("SELECT pg_advisory_unlock(hashtext('nudge-core-migrations'))")
            .execute(&mut connection)
            .await
            .map_err(|error| error.to_string())?;

        let _ = connection.close().await;

        result.map_err(|error| error.to_string())
    }

    pub async fn create_schema(connection: &mut PgConnection, schema_name: &str) {
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS \"{schema_name}\""))
            .execute(&mut *connection)
            .await
            .expect("creating the test schema");
    }

    static EXTENSION_SETUP: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    pub async fn setup_upstream_extensions(connection: &mut PgConnection) {
        let _guard = EXTENSION_SETUP.lock().await;

        sqlx::query("SELECT pg_advisory_lock(hashtext('nudge-core-migrations'))")
            .execute(&mut *connection)
            .await
            .expect("taking the extension lock");

        for extension in ["uuid-ossp", "unaccent", "citext"] {
            sqlx::query(&format!("CREATE EXTENSION IF NOT EXISTS \"{extension}\""))
                .execute(&mut *connection)
                .await
                .expect("creating the upstream extension");
        }

        sqlx::query(
            "CREATE OR REPLACE FUNCTION public.unaccent_immutable(input text)
    RETURNS text
    LANGUAGE sql
    IMMUTABLE
AS $$
SELECT public.unaccent('public.unaccent'::regdictionary, input)
$$;",
        )
        .execute(&mut *connection)
        .await
        .expect("creating the immutable unaccent wrapper");

        sqlx::query("SELECT pg_advisory_unlock(hashtext('nudge-core-migrations'))")
            .execute(&mut *connection)
            .await
            .expect("releasing the extension lock");
    }

    pub async fn drop_schema(connection: &mut PgConnection, schema_name: &str) {
        sqlx::query(&format!("DROP SCHEMA IF EXISTS \"{schema_name}\" CASCADE"))
            .execute(&mut *connection)
            .await
            .expect("dropping the test schema");
    }

    pub fn unique_schema_name(prefix: &str) -> String {
        use uuid::Uuid;

        let id = Uuid::new_v4().simple().to_string();

        format!("{prefix}_{}", &id[..12])
    }
}
