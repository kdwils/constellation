use kube::{Client, Config};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::k3s::K3s;

pub struct TestCluster {
    pub container: ContainerAsync<K3s>,
    pub client: Client,
}

impl TestCluster {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir().join(format!("k3s-test-{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir)?;

        // Copy k3s  file
        let k3s_config_source = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/k3s-config.yaml"
        );
        std::fs::copy(k3s_config_source, temp_dir.join("config.yaml"))?;

        let container = K3s::default()
            .with_conf_mount(&temp_dir)
            .with_privileged(true)
            .start()
            .await?;

        let kubeconfig_yaml = container.image().read_kube_config()?;
        let mut kubeconfig: kube::config::Kubeconfig = serde_yaml::from_str(&kubeconfig_yaml)?;

        let kube_port = container.get_host_port_ipv4(6443).await?;
        let server_url = format!("https://127.0.0.1:{}", kube_port);

        if let Some(cluster) = kubeconfig.clusters.first_mut() {
            cluster.cluster.as_mut().unwrap().server = Some(server_url);
        }

        let client = Client::try_from(
            Config::from_custom_kubeconfig(kubeconfig, &Default::default()).await?,
        )?;

        Ok(TestCluster { container, client })
    }

    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.container.stop().await?;
        Ok(())
    }
}
