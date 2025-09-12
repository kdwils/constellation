use k8s_openapi::api::{
    apps::v1::{Deployment, DeploymentSpec},
    core::v1::{
        Container, Namespace, Pod, PodSpec, PodTemplateSpec, Service, ServicePort, ServiceSpec,
    },
};
use k8s_openapi::apimachinery::pkg::{
    apis::meta::v1::{LabelSelector, ObjectMeta},
    util::intstr::IntOrString,
};
use kube::{
    Api, Client,
    api::{DeleteParams, ListParams},
};
use std::collections::BTreeMap;

pub struct TestResources {
    pub client: Client,
    pub namespace: String,
}

impl TestResources {
    pub fn new(client: Client, namespace: &str) -> Self {
        Self {
            client,
            namespace: namespace.to_string(),
        }
    }

    pub async fn create_namespace(&self) -> Result<(), Box<dyn std::error::Error>> {
        let namespace = Namespace {
            metadata: ObjectMeta {
                name: Some(self.namespace.clone()),
                ..Default::default()
            },
            ..Default::default()
        };

        let namespaces: Api<Namespace> = Api::all(self.client.clone());

        match namespaces.create(&Default::default(), &namespace).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(ae)) if ae.code == 409 => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn create_test_deployment(
        &self,
        name: &str,
        replicas: i32,
    ) -> Result<Deployment, Box<dyn std::error::Error>> {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), name.to_string());

        let deployment = Deployment {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(self.namespace.clone()),
                ..Default::default()
            },
            spec: Some(DeploymentSpec {
                replicas: Some(replicas),
                selector: LabelSelector {
                    match_labels: Some(labels.clone()),
                    ..Default::default()
                },
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: Some(labels.clone()),
                        ..Default::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: "app".to_string(),
                            image: Some("busybox:latest".to_string()),
                            command: Some(vec!["sleep".to_string(), "3600".to_string()]),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);
        Ok(deployments.create(&Default::default(), &deployment).await?)
    }

    pub async fn create_test_service(
        &self,
        name: &str,
        app_selector: &str,
    ) -> Result<Service, Box<dyn std::error::Error>> {
        self.create_test_service_with_annotations(name, app_selector, None).await
    }

    pub async fn create_test_service_with_annotations(
        &self,
        name: &str,
        app_selector: &str,
        annotations: Option<BTreeMap<String, String>>,
    ) -> Result<Service, Box<dyn std::error::Error>> {
        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), app_selector.to_string());

        let service = Service {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(self.namespace.clone()),
                annotations,
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                selector: Some(selector),
                ports: Some(vec![ServicePort {
                    port: 80,
                    target_port: Some(IntOrString::Int(80)),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let services: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        Ok(services.create(&Default::default(), &service).await?)
    }

    pub async fn wait_for_pods_ready(
        &self,
        label_selector: &str,
        expected_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        for _ in 0..120 {
            let pod_list = pods
                .list(&ListParams {
                    label_selector: Some(label_selector.to_string()),
                    ..Default::default()
                })
                .await?;

            let ready_pods = pod_list
                .items
                .iter()
                .filter(|pod| {
                    pod.status
                        .as_ref()
                        .and_then(|s| s.phase.as_ref())
                        .map(|phase| phase == "Running")
                        .unwrap_or(false)
                })
                .count();

            if ready_pods == expected_count {
                return Ok(());
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Err(format!(
            "Expected {} pods to become ready, but only found matching pods",
            expected_count
        )
        .into())
    }

    pub async fn get_pods(
        &self,
        label_selector: &str,
    ) -> Result<Vec<Pod>, Box<dyn std::error::Error>> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let pod_list = pods
            .list(&ListParams {
                label_selector: Some(label_selector.to_string()),
                ..Default::default()
            })
            .await?;

        Ok(pod_list.items)
    }

    pub async fn restart_deployment(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);
        deployments.restart(&name).await?;
        Ok(())
    }

    pub async fn create_test_pod_with_annotations(
        &self,
        name: &str,
        labels: BTreeMap<String, String>,
        annotations: Option<BTreeMap<String, String>>,
    ) -> Result<Pod, Box<dyn std::error::Error>> {
        let pod = Pod {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(self.namespace.clone()),
                labels: Some(labels),
                annotations,
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: "app".to_string(),
                    image: Some("busybox:latest".to_string()),
                    command: Some(vec!["sleep".to_string(), "3600".to_string()]),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        Ok(pods.create(&Default::default(), &pod).await?)
    }

    pub async fn delete_pod(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        pods.delete(name, &DeleteParams::default()).await?;
        Ok(())
    }

    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        let namespaces: Api<Namespace> = Api::all(self.client.clone());
        match namespaces
            .delete(&self.namespace, &DeleteParams::default())
            .await
        {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(ae)) if ae.code == 404 => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
