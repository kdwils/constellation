// mod controller;
mod router;
mod watcher;

use watcher::State;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().compact().init();

    let state = State::default();
    let router = router::new_router(state.clone()).await;
    let watchers = tokio::spawn(watcher::run(state.clone()));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .expect("couldn't bind to 8080");

    let http = axum::serve(listener, router).with_graceful_shutdown(shutdown_signal());

    let (_, server_result) = tokio::join!(http, watchers);
    server_result.unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
}
