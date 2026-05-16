//! Top-level Axum router builder.
//!
//! `router(state)` assembles every route group + middleware. Callers either
//! drive it with [`crate::serve`] (real TCP listener) or feed it to
//! [`tower::ServiceExt::oneshot`] for in-process tests.

use std::sync::Arc;

use axum::middleware;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::auth::auth_middleware;
use crate::routes;
use crate::state::AppState;

/// Build the Axum application router. All state is consumed via [`Arc`].
pub fn router(state: Arc<AppState>) -> Router {
    let cors_layer = state.cors.layer();

    let v1 = Router::<Arc<AppState>>::new()
        .nest("/instance", routes::instance::router())
        .nest("/config", routes::config::router())
        .nest("/session", routes::session::router())
        .nest("/file", routes::file::router())
        .nest("/daemon", routes::daemon::router())
        .nest("/sync", routes::sync::router())
        .nest("/tui", routes::tui::router())
        .nest("/provider", routes::provider::router())
        .nest("/permission", routes::permission::router())
        .nest("/question", routes::question::router())
        .nest("/mcp", routes::mcp::router())
        .nest("/workspace", routes::workspace::router())
        .nest("/experimental", routes::experimental::router())
        .nest("/events", routes::events::router())
        .nest("/ws", routes::ws::router())
        .nest("/pty", routes::pty::router());

    let v2 = Router::<Arc<AppState>>::new()
        .nest("/session", routes::v2::session::router())
        .nest("/session", routes::v2::message::router());

    Router::<Arc<AppState>>::new()
        .nest("/api/v1", v1)
        .nest("/api/v2", v2)
        .nest("/api", routes::openapi::router())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
