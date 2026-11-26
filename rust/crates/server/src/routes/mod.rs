pub mod health;

use axum::routing::get;
use axum::Router;

use crate::auth;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let auth_routes = auth::routes::router();

    Router::new()
        .route("/healthz", get(health::healthz))
        .merge(auth_routes)
        .with_state(state)
}
