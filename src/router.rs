use crate::watcher::{HierarchyNode, State as AppState};
use axum::{
    Router,
    extract::{
        State as AxumState, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Serialize;
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
        .route("/state/stream", get(websocket_handler))
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

async fn websocket_handler(
    ws: WebSocketUpgrade,
    AxumState(app_state): AxumState<AppState>,
) -> Response {
    tracing::info!("WebSocket client attempting to connect");
    ws.on_upgrade(move |socket| handle_socket(socket, app_state))
}

async fn handle_socket(socket: WebSocket, app_state: AppState) {
    tracing::info!("WebSocket client connected");

    let (mut sender, mut receiver) = socket.split();
    let mut rx = app_state.state_updates.subscribe();

    let initial_state = {
        let hierarchy = app_state.hierarchy.read().await;
        let mut sorted_hierarchy = hierarchy.clone();
        sorted_hierarchy.sort_by(|a, b| a.name.cmp(&b.name));
        sorted_hierarchy
    };

    let initial_json = serde_json::to_string(&initial_state).unwrap_or_else(|_| "[]".to_string());
    tracing::info!("Sending initial state to WebSocket client");

    if sender
        .send(Message::Text(initial_json.into()))
        .await
        .is_err()
    {
        tracing::warn!("Failed to send initial state to WebSocket client");
        return;
    }

    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(mut state) => {
                    tracing::debug!("Received broadcast message, sending to WebSocket client");
                    state.sort_by(|a, b| a.name.cmp(&b.name));
                    match serde_json::to_string(&state) {
                        Ok(json) => {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                tracing::info!("WebSocket client disconnected");
                                break;
                            }
                        }
                        Err(err) => {
                            tracing::warn!("Failed to serialize state for WebSocket: {}", err);
                            if sender
                                .send(Message::Text("{\"error\":\"serialization_failed\"}".into()))
                                .await
                                .is_err()
                            {
                                tracing::info!("WebSocket client disconnected");
                                break;
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::debug!("Stream lagged by {} messages, sending current state", n);
                    let hierarchy = app_state.hierarchy.read().await;
                    let mut sorted_hierarchy = hierarchy.clone();
                    sorted_hierarchy.sort_by(|a, b| a.name.cmp(&b.name));

                    match serde_json::to_string(&sorted_hierarchy) {
                        Ok(json) => {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                tracing::info!("WebSocket client disconnected");
                                break;
                            }
                        }
                        Err(err) => {
                            tracing::warn!("Failed to serialize current state after lag: {}", err);
                            if sender
                                .send(Message::Text("{\"error\":\"serialization_failed\"}".into()))
                                .await
                                .is_err()
                            {
                                tracing::info!("WebSocket client disconnected");
                                break;
                            }
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::error!("Broadcast channel closed, ending WebSocket stream");
                    break;
                }
            }
        }
        tracing::info!("WebSocket send task ended");
    });

    let mut recv_task = tokio::spawn(async move {
while let Some(Ok(Message::Close(_))) = receiver.next().await {
    tracing::info!("WebSocket client sent close message");
}
    });

    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        },
        _ = (&mut recv_task) => {
            send_task.abort();
        },
    }

    tracing::info!("WebSocket connection closed");
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
