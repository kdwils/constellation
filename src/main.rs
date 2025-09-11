// mod controller;
pub mod router;
pub mod server;
pub mod watcher;

use server::ConstellationServer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().compact().init();

    let server = ConstellationServer::new("0.0.0.0:8080")
        .await
        .expect("Failed to start server");

    server.serve().await.expect("Server error");
}
