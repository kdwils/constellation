use crate::watcher::{HierarchyNode, State as AppState};
use axum::{
    Router,
    extract::State as AxumState,
    response::{Html, Json},
    routing::get,
};
use serde::Serialize;
use tower::ServiceBuilder;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Serialize)]
struct HealthCheck {
    message: String,
}

pub async fn new_router(app_state: AppState) -> Router {
    let file_service = ServeDir::new("frontend/dist");
    let index_service = ServeFile::new("frontend/dist/index.html");

    Router::new()
        .route("/healthz", get(healthz))
        .route("/state", get(state))
        .route_service("/", index_service)
        .fallback_service(file_service)
        .with_state(app_state)
}

async fn state(AxumState(app_state): AxumState<AppState>) -> Json<Vec<HierarchyNode>> {
    let graph = app_state.hierarchy.read().await;
    Json(graph.clone())
}

async fn healthz() -> Json<HealthCheck> {
    Json(HealthCheck {
        message: "ok".into(),
    })
}
