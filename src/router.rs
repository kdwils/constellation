use crate::watcher::{HierarchyNode, State as AppState};
use axum::{
    Router,
    extract::State as AxumState,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
};
use serde::Serialize;
use tokio_stream::wrappers::BroadcastStream;
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
        .route("/state/stream", get(state_stream))
        .route_service("/", index_service)
        .fallback_service(file_service)
        .with_state(app_state)
}

async fn state(AxumState(app_state): AxumState<AppState>) -> Json<Vec<HierarchyNode>> {
    let graph = app_state.hierarchy.read().await;
    let mut sorted_graph = graph.clone();
    sorted_graph.sort_by(|a, b| a.name.cmp(&b.name));
    Json(sorted_graph)
}

async fn state_stream(AxumState(app_state): AxumState<AppState>) -> Response {
    use axum::http::header;
    use futures::stream;
    use tokio_stream::StreamExt;

    // Send initial state
    let initial_state = {
        let hierarchy = app_state.hierarchy.read().await;
        let mut sorted_hierarchy = hierarchy.clone();
        sorted_hierarchy.sort_by(|a, b| a.name.cmp(&b.name));
        sorted_hierarchy
    };

    let initial_json = serde_json::to_string(&initial_state).unwrap_or_else(|_| "[]".to_string());
    let initial_event = format!("data: {}\n\n", initial_json);

    // Subscribe to updates
    let rx = app_state.state_updates.subscribe();
    let update_stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(mut state) => {
            state.sort_by(|a, b| a.name.cmp(&b.name));
            match serde_json::to_string(&state) {
                Ok(json) => Some(format!("data: {}\n\n", json)),
                Err(_) => None,
            }
        }
        Err(_) => None,
    });

    // Combine initial state + updates
    let combined_stream = stream::once(async { initial_event })
        .chain(update_stream)
        .map(Ok::<_, axum::Error>);

    let body = axum::body::Body::from_stream(combined_stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .body(body)
        .unwrap()
}

async fn healthz(AxumState(app_state): AxumState<AppState>) -> Response {
    use axum::http::StatusCode;

    let hierarchy = app_state.hierarchy.read().await;
    let ready = !hierarchy.is_empty();
    drop(hierarchy);

    if !ready {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthCheck {
                message: "waiting for kubernetes resources".into(),
            }),
        )
            .into_response();
    }

    return (
        StatusCode::OK,
        Json(HealthCheck {
            message: "ready".into(),
        }),
    )
        .into_response();
}
