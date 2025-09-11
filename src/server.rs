use crate::{router, watcher, watcher::State};
use kube::Client;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

pub struct ConstellationServer {
    pub state: State,
    pub addr: SocketAddr,
    watcher_handle: JoinHandle<()>,
    server_handle: JoinHandle<Result<(), std::io::Error>>,
}

impl ConstellationServer {
    pub async fn new(bind_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let state = State::default();

        let watcher_state = state.clone();
        let watcher_handle = tokio::spawn(async move {
            watcher::run(watcher_state).await;
        });

        let server_state = state.clone();
        let router = router::new_router(server_state).await;
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move { axum::serve(listener, router).await });

        Ok(ConstellationServer {
            state,
            addr,
            watcher_handle,
            server_handle,
        })
    }

    pub async fn new_with_client(
        bind_addr: &str,
        client: Client,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let state = State::default();

        let watcher_state = state.clone();
        let watcher_client = client.clone();
        let watcher_handle = tokio::spawn(async move {
            watcher::run_with_client(watcher_state, watcher_client).await;
        });

        let server_state = state.clone();
        let router = router::new_router(server_state).await;
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move { axum::serve(listener, router).await });

        Ok(ConstellationServer {
            state,
            addr,
            watcher_handle,
            server_handle,
        })
    }

    pub async fn run_until_shutdown(self) -> Result<(), Box<dyn std::error::Error>> {
        let (server_result, watcher_result) = tokio::join!(self.server_handle, self.watcher_handle);
        match server_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e.into()),
            Err(e) => return Err(e.into()),
        }
        watcher_result?;
        Ok(())
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn shutdown(self) {
        self.watcher_handle.abort();
        self.server_handle.abort();
    }
}
