use testcontainers::{ContainerAsync, runners::AsyncRunner, ImageExt};
use testcontainers_modules::k3s::K3s;
use kube::{Client, Config};

pub struct TestCluster {
    pub container: ContainerAsync<K3s>,
    pub client: Client,
    pub kubeconfig: String,
}

impl TestCluster {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let container = K3s::default().start().await?;
    
        let kubeconfig_yaml = container.image().read_kube_config()?;
        let kubeconfig: kube::config::Kubeconfig = serde_yaml::from_str(&kubeconfig_yaml)?;
        let client = Client::try_from(Config::from_custom_kubeconfig(kubeconfig, &Default::default()).await?)?;
        
        Ok(TestCluster {
            container,
            client,
            kubeconfig: kubeconfig_yaml,
        })
    }
    
    pub async fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        use kube::api::Api;
        use k8s_openapi::api::core::v1::Node;
        
        let nodes: Api<Node> = Api::all(self.client.clone());
        
        for _ in 0..60 {
            match nodes.list(&Default::default()).await {
                Ok(node_list) if !node_list.items.is_empty() => {
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
        
        Err("Cluster did not become ready in time".into())
    }
}