use crate::controller::{NamespaceHierarchy, State as AppState};
use axum::{Router, extract::State as AxumState, response::Json, routing::get};
use serde::Serialize;

#[derive(Serialize)]
struct HealthCheck {
    message: String,
}

pub async fn new_router(app_state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/pods", get(pods))
        .with_state(app_state)
}

// #[axum::debug_handler]
async fn pods(AxumState(app_state): AxumState<AppState>) -> Json<Vec<NamespaceHierarchy>> {
    let graph = app_state.graph.read().await;
    let chains = graph.namespace_hierarchy();
    Json(chains)
}

async fn healthz() -> Json<HealthCheck> {
    Json(HealthCheck {
        message: "ok".into(),
    })
}
