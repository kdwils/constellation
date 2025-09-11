use crate::{router, watcher, watcher::State};
use kube::Client;
use std::net::SocketAddr;

pub struct ConstellationServer {
    pub state: State,
    pub addr: SocketAddr,
    pub listener: tokio::net::TcpListener,
    pub router: axum::Router,
    pub client: Client,
}

impl ConstellationServer {
    pub async fn new(bind_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::try_default().await?;
        Self::new_with_client(bind_addr, client).await
    }

    pub async fn new_with_client(
        bind_addr: &str,
        client: Client,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let state = State::default();
        let router = router::new_router(state.clone()).await;
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        let addr = listener.local_addr()?;

        Ok(ConstellationServer {
            state,
            addr,
            listener,
            router,
            client,
        })
    }

    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let watcher_state = self.state.clone();
        let watcher_client = self.client.clone();
        let _watcher_handle = tokio::spawn(async move {
            println!("Starting watcher...");
            watcher::run_with_client(watcher_state, watcher_client).await;
            println!("Watcher finished");
        });

        axum::serve(self.listener, self.router).await?;
        Ok(())
    }
}
