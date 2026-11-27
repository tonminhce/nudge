use serde_json::Value;
use uuid::Uuid;

use crate::state::AppState;
use nudge_core::NodeEnv;

pub(crate) static DB_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(crate) struct SeededWorkspace {
    pub state: AppState,
    pub workspace_id: Uuid,
    pub schema_name: String,
}

pub(crate) fn test_config(pg_database_url: String) -> nudge_core::Config {
    nudge_core::Config {
        node_env: NodeEnv::Test,
        node_port: 3996,
        server_url: "http://localhost:3996".to_string(),
        pg_database_url,
        pg_pool_max_connections: 4,
        redis_url: "redis://127.0.0.1:6397".to_string(),
        redis_cache_prefix: String::new(),
        app_secret: "aggregate-db-test-secret".to_string(),
        encryption_key: None,
        fallback_encryption_key: None,
        access_token_expires_in_secs: 1800,
        refresh_token_expires_in_secs: 3600,
        login_token_expires_in_secs: 900,
        workspace_agnostic_token_expires_in_secs: 1800,
        refresh_token_reuse_grace_period_secs: 60,
        frontend_url: "http://localhost:3996".to_string(),
        auth_cookie_same_site: "lax".to_string(),
        auth_cookie_allowed_origins: String::new(),
        session_absolute_lifetime_secs: 3600,
        session_idle_timeout_secs: 3600,
        is_cloudflare_geolocation_enabled: false,
        workspace_schema_ddl_locked: false,
        search_ilike_fallback_timeout_ms: 2000,
        allow_requests_to_twenty_icons: true,
    }
}

pub(crate) async fn seeded_workspace(url: &str) -> Option<SeededWorkspace> {
    let state = AppState::connect(&test_config(url.to_string()))
        .await
        .expect("aggregate db test state");

    crate::workspace_migrations::ddl::test_support::migrate_core(url)
        .await
        .expect("aggregate core migrations");

    let workspace_id = Uuid::new_v4();
    let schema_name = nudge_core::ids::workspace_schema_name(&workspace_id);
    let application_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    let mut fixture = state.core_pool.begin().await.expect("aggregate fixture");

    sqlx::query(
        "INSERT INTO core.\"workspace\" \
         (id, \"displayName\", \"inviteHash\", \"activationStatus\", subdomain, \
          \"workspaceCustomApplicationId\", \"isPasswordAuthEnabled\", \"databaseSchema\") \
         VALUES ($1, 'Aggregate Parity', $2, 'PENDING_CREATION', $3, $4, true, $5)",
    )
    .bind(workspace_id)
    .bind(Uuid::new_v4())
    .bind(format!(
        "aggregate-{}",
        &workspace_id.simple().to_string()[..8]
    ))
    .bind(application_id)
    .bind(&schema_name)
    .execute(&mut *fixture)
    .await
    .expect("insert aggregate workspace");

    sqlx::query(
        "INSERT INTO core.\"application\" \
         (id, \"universalIdentifier\", name, \"sourceType\", state, \"sourcePath\", \"workspaceId\") \
         VALUES ($1, $2, 'Aggregate Application', 'LOCAL', 'INSTALLED', '', $3)",
    )
    .bind(application_id)
    .bind(Uuid::new_v4())
    .bind(workspace_id)
    .execute(&mut *fixture)
    .await
    .expect("insert aggregate application");

    sqlx::query(
        "INSERT INTO core.\"role\" \
         (id, \"universalIdentifier\", label, \"applicationId\", \"workspaceId\") \
         VALUES ($1, $2, 'Member', $3, $4)",
    )
    .bind(role_id)
    .bind(Uuid::new_v4())
    .bind(application_id)
    .bind(workspace_id)
    .execute(&mut *fixture)
    .await
    .expect("insert aggregate role");

    sqlx::query(
        "UPDATE core.\"workspace\" SET \"activationStatus\" = 'CREATED', \
            \"defaultRoleId\" = $2 WHERE id = $1",
    )
    .bind(workspace_id)
    .bind(role_id)
    .execute(&mut *fixture)
    .await
    .expect("activate aggregate workspace");

    fixture.commit().await.expect("commit aggregate fixture");

    sqlx::query(&format!("CREATE SCHEMA \"{schema_name}\""))
        .execute(&state.core_pool)
        .await
        .expect("create aggregate schema");

    let outcome = crate::standard_objects::seed_standard_objects(&state, workspace_id)
        .await
        .expect("seed aggregate standard objects");
    assert!(outcome.seeded);

    Some(SeededWorkspace {
        state,
        workspace_id,
        schema_name,
    })
}

pub(crate) fn record(json: &str) -> serde_json::Map<String, Value> {
    serde_json::from_str::<Value>(json)
        .expect("record json")
        .as_object()
        .expect("record object")
        .clone()
}

pub(crate) async fn runner_schema(seeded: &SeededWorkspace) -> crate::query::runner::RunnerSchema {
    let maps = crate::metadata::flat::load_workspace_flat_maps(
        &seeded.state.core_pool,
        seeded.workspace_id,
    )
    .await
    .expect("load aggregate flat maps");
    let objects = maps
        .object_metadata_maps
        .by_universal_identifier
        .values()
        .cloned()
        .collect::<Vec<_>>();

    crate::query::runner::RunnerSchema::build(
        &seeded.workspace_id,
        &objects,
        &maps.field_metadata_maps,
        &maps.index_metadata_maps,
    )
    .expect("build aggregate schema")
}

pub(crate) async fn drop_workspace(seeded: &SeededWorkspace) {
    sqlx::query(&format!("DROP SCHEMA \"{}\" CASCADE", seeded.schema_name))
        .execute(&seeded.state.core_pool)
        .await
        .expect("drop aggregate schema");
}
