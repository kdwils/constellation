use crate::watcher::{HierarchyNode, State as AppState};
use axum::{
    Router,
    extract::State as AxumState,
    http::StatusCode,
    response::{
        IntoResponse, Json, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
};
use futures::{Stream, stream};
use serde::Serialize;
use std::convert::Infallible;
use tokio_stream::StreamExt;
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

async fn state_stream(
    AxumState(app_state): AxumState<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = app_state.state_updates.subscribe();

    let initial_state = {
        let hierarchy = app_state.hierarchy.read().await;
        let mut sorted_hierarchy = hierarchy.clone();
        sorted_hierarchy.sort_by(|a, b| a.name.cmp(&b.name));
        sorted_hierarchy
    };

    let initial_json = serde_json::to_string(&initial_state).unwrap_or_else(|_| "[]".to_string());

    let initial_event = stream::once(async { Ok(Event::default().data(initial_json)) });

    let update_stream = async_stream::stream! {
        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(mut state) => {
                    state.sort_by(|a, b| a.name.cmp(&b.name));
                    match serde_json::to_string(&state) {
                        Ok(json) => yield Ok(Event::default().data(json)),
                        Err(err) => {
                            tracing::warn!("Failed to serialize state for SSE: {}", err);
                            yield Ok(Event::default().data("{\"error\":\"serialization_failed\"}"));
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::debug!("Stream lagged by {} messages, sending current state", n);
                    let hierarchy = app_state.hierarchy.read().await;
                    let mut sorted_hierarchy = hierarchy.clone();
                    sorted_hierarchy.sort_by(|a, b| a.name.cmp(&b.name));

                    match serde_json::to_string(&sorted_hierarchy) {
                        Ok(json) => yield Ok(Event::default().data(json)),
                        Err(err) => {
                            tracing::warn!("Failed to serialize current state after lag: {}", err);
                            yield Ok(Event::default().data("{\"error\":\"serialization_failed\"}"));
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::error!("Broadcast channel closed, ending SSE stream");
                    break;
                }
            }
        }
    };

    let combined_stream = initial_event.chain(update_stream);
    Sse::new(combined_stream).keep_alive(KeepAlive::default())
}

async fn healthz(AxumState(app_state): AxumState<AppState>) -> Response {
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

    (
        StatusCode::OK,
        Json(HealthCheck {
            message: "ready".into(),
        }),
    )
        .into_response()
}
