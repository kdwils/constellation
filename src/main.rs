// mod controller;
pub mod router;
pub mod server;
pub mod watcher;

use server::Server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().compact().init();

    let server = Server::new("0.0.0.0:8080")
        .await
        .expect("Failed to start server");

    server.serve().await.expect("Server error");
}
