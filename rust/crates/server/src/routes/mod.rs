pub mod health;

use axum::middleware;
use axum::routing::get;
use axum::Router;

use crate::auth;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let auth_routes = auth::routes::router();

    Router::new()
        .route("/healthz", get(health::healthz))
        .merge(auth_routes)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::csrf::cookie_session_csrf,
        ))
        .with_state(state)
}
