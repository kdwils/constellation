use futures::{Stream, StreamExt};
use k8s_openapi::api::core::v1;
use kube::{
    Client, ResourceExt,
    api::Api,
    runtime::{
        WatchStreamExt,
        reflector::{self, Lookup, Store},
        watcher::{self, Error as WatcherError},
    },
};

use std::{collections::BTreeMap, collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

use gateway_api::httproutes::{HTTPRoute, HTTPRouteSpec};
use k8s_openapi::api::core::v1::{Namespace, Pod, Service, ServicePort};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::api::ObjectMeta;
use serde::Serialize;
use tracing::{error, info};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ResourceKind {
    Namespace,
    Service,
    Pod,
    HTTPRoute,
}

impl std::fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceKind::HTTPRoute => write!(f, "HTTPRoute"),
            ResourceKind::Service => write!(f, "Service"),
            ResourceKind::Pod => write!(f, "Pod"),
            ResourceKind::Namespace => write!(f, "Namespace"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResourceSpec {
    Namespace(()),
    Service(Box<v1::ServiceSpec>),
    Pod(Box<v1::PodSpec>),
    HTTPRoute(HTTPRouteSpec),
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerPortInfo {
    pub port: u32,
    pub name: Option<String>,
    pub protocol: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ServicePortInfo {
    pub service_ports: Vec<u32>,
    pub port_mappings: Vec<String>,
    pub target_ports: Vec<u32>,
    pub target_port_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostnames: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selectors: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port_mappings: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ports: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_port_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_refs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cluster_ips: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ips: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pod_ips: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_ports: Option<Vec<ContainerPortInfo>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HierarchyNode {
    pub kind: ResourceKind,
    pub name: String,
    pub relatives: Vec<HierarchyNode>,
    #[serde(skip)]
    pub metadata: ObjectMeta,
    #[serde(skip)]
    pub spec: Option<ResourceSpec>,
    #[serde(flatten)]
    pub resource_metadata: ResourceMetadata,
}

#[derive(Clone)]
pub struct State {
    pub hierarchy: Arc<RwLock<Vec<HierarchyNode>>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            hierarchy: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[derive(Clone)]
pub struct Context {
    state: State,
    pod_store: Store<v1::Pod>,
    service_store: Store<v1::Service>,
    namespace_store: Store<Namespace>,
    httproute_store: Store<HTTPRoute>,
}

pub async fn run(state: State) {
    let client = Client::try_default()
        .await
        .expect("failed to create kubernetes client");
    run_with_client(state, client).await;
}

pub async fn run_with_client(state: State, client: Client) {
    let config = watcher::Config::default();

    let pod_api: Api<v1::Pod> = Api::all(client.clone());
    let (pod_store, pod_writer) = reflector::store::<v1::Pod>();
    let pod_rf = reflector::reflector(
        pod_writer,
        watcher::watcher(pod_api, config.clone()).default_backoff(),
    );

    let namespace_api: Api<Namespace> = Api::all(client.clone());
    let (namespace_store, namespace_writer) = reflector::store::<Namespace>();
    let namespace_rf = reflector::reflector(
        namespace_writer,
        watcher::watcher(namespace_api, config.clone()).default_backoff(),
    );

    let service_api: Api<v1::Service> = Api::all(client.clone());
    let (service_store, service_writer) = reflector::store::<v1::Service>();
    let service_rf = reflector::reflector(
        service_writer,
        watcher::watcher(service_api, config.clone()).default_backoff(),
    );

    let httproute_api: Api<HTTPRoute> = Api::all(client.clone());
    let (httproute_store, httproute_writer) = reflector::store::<HTTPRoute>();
    let httproute_rf = reflector::reflector(
        httproute_writer,
        watcher::watcher(httproute_api, config.clone()).default_backoff(),
    );

    let ctx: Context = Context {
        state,
        pod_store,
        service_store,
        namespace_store,
        httproute_store,
    };

    let pod_stream = Box::pin(pod_rf);
    let service_stream = Box::pin(service_rf);
    let namespace_stream = Box::pin(namespace_rf);
    let httproute_stream = Box::pin(httproute_rf);

    tokio::spawn(pod_watcher(ctx.clone(), pod_stream));
    tokio::spawn(service_watcher(ctx.clone(), service_stream));
    tokio::spawn(namespace_watcher(ctx.clone(), namespace_stream));
    tokio::spawn(httproute_watcher(ctx.clone(), httproute_stream));

    ctx.pod_store.wait_until_ready().await.unwrap();
    ctx.service_store.wait_until_ready().await.unwrap();
    ctx.namespace_store.wait_until_ready().await.unwrap();
    ctx.httproute_store.wait_until_ready().await.unwrap();

    build_initial_relationships(ctx.clone()).await;
}

fn extract_resource_metadata(
    kind: &ResourceKind,
    metadata: &ObjectMeta,
    spec: &Option<ResourceSpec>,
) -> ResourceMetadata {
    match kind {
        ResourceKind::HTTPRoute => {
            let (hostnames, backend_refs) = match spec {
                Some(ResourceSpec::HTTPRoute(spec)) => {
                    let mut hosts = Vec::new();
                    if let Some(hostname_list) = &spec.hostnames {
                        for hostname in hostname_list {
                            hosts.push(hostname.clone());
                        }
                    }
                    let hostnames = if hosts.is_empty() { None } else { Some(hosts) };

                    let mut backends = Vec::new();
                    for rule in spec.rules.iter().flatten() {
                        for backend_ref in rule.backend_refs.iter().flatten() {
                            if let Some(kind) = &backend_ref.kind
                                && kind == &ResourceKind::Service.to_string()
                            {
                                backends.push(backend_ref.name.clone());
                            }
                        }
                    }
                    let backend_refs = if backends.is_empty() {
                        None
                    } else {
                        Some(backends)
                    };

                    (hostnames, backend_refs)
                }
                _ => (None, None),
            };
            ResourceMetadata {
                hostnames,
                selectors: None,
                ports: None,
                port_mappings: None,
                target_ports: None,
                target_port_names: None,
                labels: None,
                phase: None,
                backend_refs,
                service_type: None,
                cluster_ips: None,
                external_ips: None,
                pod_ips: None,
                container_ports: None,
            }
        }
        ResourceKind::Service => {
            let (
                selectors,
                ports,
                port_mappings,
                target_ports,
                target_port_names,
                service_type,
                cluster_ips,
                external_ips,
            ) = match spec {
                Some(ResourceSpec::Service(spec)) => {
                    let selectors = spec.selector.clone();
                    let (ports, port_mappings, target_ports, target_port_names) = spec
                        .ports
                        .as_ref()
                        .map(|port_list| {
                            let port_info = extract_port_info(port_list);
                            (
                                Some(port_info.service_ports),
                                Some(port_info.port_mappings),
                                Some(port_info.target_ports),
                                Some(port_info.target_port_names),
                            )
                        })
                        .unwrap_or((None, None, None, None));

                    let service_type = spec.type_.clone();

                    let mut cluster_ips = Vec::new();
                    if let Some(ip) = &spec.cluster_ip
                        && !ip.is_empty()
                        && ip != "None"
                    {
                        cluster_ips.push(ip.clone());
                    }
                    if let Some(ips) = &spec.cluster_ips {
                        for ip in ips {
                            if !ip.is_empty() && ip != "None" && !cluster_ips.contains(ip) {
                                cluster_ips.push(ip.clone());
                            }
                        }
                    }
                    let cluster_ips = match cluster_ips.is_empty() {
                        true => None,
                        false => Some(cluster_ips),
                    };

                    let external_ips = spec.external_ips.clone().filter(|ips| !ips.is_empty());
                    (
                        selectors,
                        ports,
                        port_mappings,
                        target_ports,
                        target_port_names,
                        service_type,
                        cluster_ips,
                        external_ips,
                    )
                }
                _ => (None, None, None, None, None, None, None, None),
            };
            ResourceMetadata {
                hostnames: None,
                selectors,
                ports,
                port_mappings,
                target_ports,
                target_port_names,
                labels: None,
                phase: None,
                backend_refs: None,
                service_type,
                cluster_ips,
                external_ips,
                pod_ips: None,
                container_ports: None,
            }
        }
        ResourceKind::Pod => {
            let labels = metadata.labels.clone();
            let (ports, container_ports) = match spec {
                Some(ResourceSpec::Pod(spec)) => {
                    let mut port_list = Vec::new();
                    let mut container_port_list = Vec::new();

                    for container in &spec.containers {
                        if let Some(container_ports) = &container.ports {
                            for port in container_ports {
                                port_list.push(port.container_port as u32);
                                container_port_list.push(ContainerPortInfo {
                                    port: port.container_port as u32,
                                    name: port.name.clone(),
                                    protocol: port.protocol.clone(),
                                });
                            }
                        }
                    }

                    let ports = match port_list.is_empty() {
                        true => None,
                        false => Some(port_list),
                    };
                    let container_ports = match container_port_list.is_empty() {
                        true => None,
                        false => Some(container_port_list),
                    };

                    (ports, container_ports)
                }
                _ => (None, None),
            };
            ResourceMetadata {
                hostnames: None,
                selectors: None,
                ports,
                port_mappings: None,
                target_ports: None,
                target_port_names: None,
                labels,
                phase: None,
                backend_refs: None,
                service_type: None,
                cluster_ips: None,
                external_ips: None,
                pod_ips: None,
                container_ports,
            }
        }
        ResourceKind::Namespace => ResourceMetadata {
            hostnames: None,
            selectors: None,
            ports: None,
            port_mappings: None,
            target_ports: None,
            target_port_names: None,
            labels: None,
            phase: None,
            backend_refs: None,
            service_type: None,
            cluster_ips: None,
            external_ips: None,
            pod_ips: None,
            container_ports: None,
        },
    }
}

fn new_pod(pod: &Pod) -> HierarchyNode {
    let spec = pod.spec.clone().map(|s| ResourceSpec::Pod(Box::new(s)));
    let metadata = pod.metadata.clone();
    let mut resource_metadata = extract_resource_metadata(&ResourceKind::Pod, &metadata, &spec);

    if let Some(status) = &pod.status {
        resource_metadata.phase = status.phase.clone();

        let mut pod_ips = Vec::new();
        if let Some(ip) = &status.pod_ip
            && !ip.is_empty()
        {
            pod_ips.push(ip.clone());
        }
        if let Some(ip_list) = &status.pod_ips {
            for pod_ip_obj in ip_list {
                let ip = &pod_ip_obj.ip;
                if !ip.is_empty() && !pod_ips.contains(ip) {
                    pod_ips.push(ip.clone());
                }
            }
        }
        resource_metadata.pod_ips = match pod_ips.is_empty() {
            true => None,
            false => Some(pod_ips),
        };
    }

    HierarchyNode {
        kind: ResourceKind::Pod,
        name: pod.metadata.name.clone().unwrap_or_default(),
        relatives: Vec::new(),
        metadata,
        spec,
        resource_metadata,
    }
}

fn extract_port_info(ports: &[ServicePort]) -> ServicePortInfo {
    let service_ports: Vec<u32> = ports.iter().map(|p| p.port as u32).collect();

    let port_mappings: Vec<String> = ports
        .iter()
        .map(|p| match &p.target_port {
            Some(IntOrString::Int(i)) => {
                if *i == p.port {
                    return p.port.to_string();
                }
                format!("{}→{}", p.port, i)
            }
            Some(IntOrString::String(s)) => format!("{}→{}", p.port, s),
            None => p.port.to_string(),
        })
        .collect();

    let target_ports: Vec<u32> = ports
        .iter()
        .map(|p| match &p.target_port {
            Some(IntOrString::Int(i)) => *i as u32,
            Some(IntOrString::String(_)) => p.port as u32,
            None => p.port as u32,
        })
        .collect();

    let target_port_names: Vec<String> = ports
        .iter()
        .filter_map(|p| match &p.target_port {
            Some(IntOrString::String(s)) => Some(s.clone()),
            _ => None,
        })
        .collect();

    ServicePortInfo {
        service_ports,
        port_mappings,
        target_ports,
        target_port_names,
    }
}

fn extract_load_balancer_ips(service: &Service) -> Vec<String> {
    println!(
        "Checking service {} for LoadBalancer IPs",
        service.name().unwrap_or_default()
    );

    let Some(status) = &service.status else {
        println!(
            "No status for service {}",
            service.name().unwrap_or_default()
        );
        return Vec::new();
    };

    let Some(load_balancer) = &status.load_balancer else {
        println!(
            "No load_balancer for service {}",
            service.name().unwrap_or_default()
        );
        return Vec::new();
    };

    let Some(ingress_list) = &load_balancer.ingress else {
        println!(
            "No ingress list for service {}",
            service.name().unwrap_or_default()
        );
        return Vec::new();
    };

    let ips: Vec<String> = ingress_list
        .iter()
        .filter_map(|ingress| ingress.ip.as_ref().or(ingress.hostname.as_ref()))
        .cloned()
        .collect();

    println!(
        "Found {} IPs for service {}: {:?}",
        ips.len(),
        service.name().unwrap_or_default(),
        ips
    );
    ips
}

fn new_service(service: &Service) -> HierarchyNode {
    let spec = service
        .spec
        .clone()
        .map(|s| ResourceSpec::Service(Box::new(s)));
    let metadata = service.metadata.clone();
    let mut resource_metadata = extract_resource_metadata(&ResourceKind::Service, &metadata, &spec);

    let lb_ips = extract_load_balancer_ips(service);
    if !lb_ips.is_empty() {
        resource_metadata.external_ips = Some(lb_ips);
    }

    HierarchyNode {
        kind: ResourceKind::Service,
        name: service.metadata.name.clone().unwrap_or_default(),
        relatives: Vec::new(),
        metadata,
        spec,
        resource_metadata,
    }
}

fn remove_pod_node(node: &mut HierarchyNode, pod_name: &str, pod_ns: Option<&str>) {
    node.relatives.retain(|p| {
        !(p.kind == ResourceKind::Pod
            && p.name == pod_name
            && p.metadata.namespace.as_deref() == pod_ns)
    });

    for child in node.relatives.iter_mut() {
        remove_pod_node(child, pod_name, pod_ns);
    }
}

fn remove_service_node(node: &mut HierarchyNode, service_name: &str, service_ns: Option<&str>) {
    node.relatives.retain(|s| {
        !(s.kind == ResourceKind::Service
            && s.name == service_name
            && s.metadata.namespace.as_deref() == service_ns)
    });

    for child in node.relatives.iter_mut() {
        remove_service_node(child, service_name, service_ns);
    }
}

fn remove_httproute_node(
    node: &mut HierarchyNode,
    httproute_name: &str,
    httproute_ns: Option<&str>,
) {
    node.relatives.retain(|h| {
        !(h.kind == ResourceKind::HTTPRoute
            && h.name == httproute_name
            && h.metadata.namespace.as_deref() == httproute_ns)
    });

    for child in node.relatives.iter_mut() {
        remove_httproute_node(child, httproute_name, httproute_ns);
    }
}

fn update_service_relationships(hierarchy: &mut [HierarchyNode], service: &Service, pods: &[Pod]) {
    let service_name = service.name().unwrap_or_default();
    let service_ns = service.metadata.namespace.as_deref();
    let service_node = new_service(service);

    for node in hierarchy.iter_mut() {
        remove_service_node(node, service_name.as_ref(), service_ns);
    }

    for namespace_node in hierarchy.iter_mut() {
        if namespace_node.kind == ResourceKind::Namespace
            && namespace_node.metadata.name.as_deref() == service_ns
        {
            let mut service_added_to_httproute = false;

            for httproute in namespace_node.relatives.iter_mut() {
                if httproute.kind == ResourceKind::HTTPRoute
                    && let Some(ResourceSpec::HTTPRoute(spec)) = &httproute.spec
                {
                    let referenced = spec
                        .rules
                        .iter()
                        .flatten()
                        .flat_map(|rule| &rule.backend_refs)
                        .flatten()
                        .any(|r| {
                            r.kind.as_deref() == Some(&ResourceKind::Service.to_string())
                                && r.name == service_name.as_ref()
                        });

                    if referenced {
                        let mut new_service = service_node.clone();

                        if let Some(ResourceSpec::Service(service_spec)) = &service_node.spec {
                            new_service.relatives.extend(
                                pods.iter()
                                    .filter(|pod| {
                                        let pod_ns = pod.metadata.namespace.as_deref();
                                        pod_ns == service_ns
                                            && selectors_match(
                                                &service_spec.selector.clone().unwrap_or_default(),
                                                pod.labels(),
                                            )
                                    })
                                    .map(new_pod),
                            );
                        }

                        httproute.relatives.push(new_service);
                        service_added_to_httproute = true;
                    }
                }
            }

            if !service_added_to_httproute {
                let mut new_service = service_node.clone();

                // Add matching pods to the service
                if let Some(ResourceSpec::Service(service_spec)) = &service_node.spec {
                    new_service.relatives.extend(
                        pods.iter()
                            .filter(|pod| {
                                let pod_ns = pod.metadata.namespace.as_deref();
                                pod_ns == service_ns
                                    && selectors_match(
                                        &service_spec.selector.clone().unwrap_or_default(),
                                        pod.labels(),
                                    )
                            })
                            .map(new_pod),
                    );
                }

                namespace_node.relatives.push(new_service);
            }
            break;
        }
    }
}

fn update_httproute_relationships(
    hierarchy: &mut [HierarchyNode],
    httproute: &HTTPRoute,
    services: &[Service],
    pods: &[Pod],
) {
    let httproute_name = httproute.name().unwrap_or_default();
    let httproute_ns = httproute.metadata.namespace.as_deref();

    for node in hierarchy.iter_mut() {
        remove_httproute_node(node, httproute_name.as_ref(), httproute_ns);
    }

    for namespace_node in hierarchy.iter_mut() {
        if namespace_node.kind == ResourceKind::Namespace
            && namespace_node.metadata.name.as_deref() == httproute_ns
        {
            let metadata = httproute.metadata.clone();
            let spec = Some(ResourceSpec::HTTPRoute(httproute.spec.clone()));
            let resource_metadata =
                extract_resource_metadata(&ResourceKind::HTTPRoute, &metadata, &spec);

            let mut httproute_node = HierarchyNode {
                kind: ResourceKind::HTTPRoute,
                name: httproute_name.as_ref().to_string(),
                relatives: Vec::new(),
                metadata,
                spec,
                resource_metadata,
            };

            if let Some(ResourceSpec::HTTPRoute(spec)) = &httproute_node.spec {
                for service in services.iter() {
                    let service_name = service.name().unwrap_or_default();
                    let service_ns = service.metadata.namespace.as_deref();

                    if service_ns != httproute_ns {
                        continue;
                    }

                    let referenced = spec
                        .rules
                        .iter()
                        .flatten()
                        .flat_map(|rule| &rule.backend_refs)
                        .flatten()
                        .any(|r| {
                            r.kind.as_deref() == Some(&ResourceKind::Service.to_string())
                                && r.name == service_name.as_ref()
                        });

                    if referenced {
                        let mut service_node = new_service(service);

                        if let Some(ResourceSpec::Service(service_spec)) = &service_node.spec {
                            service_node.relatives.extend(
                                pods.iter()
                                    .filter(|pod| {
                                        let pod_ns = pod.metadata.namespace.as_deref();
                                        pod_ns == service_ns
                                            && selectors_match(
                                                &service_spec.selector.clone().unwrap_or_default(),
                                                pod.labels(),
                                            )
                                    })
                                    .map(new_pod),
                            );
                        }

                        httproute_node.relatives.push(service_node);
                    }
                }
            }

            namespace_node.relatives.push(httproute_node);
            break;
        }
    }
}

fn update_pod_relationships(hierarchy: &mut [HierarchyNode], pod: &Pod) {
    let pod_name = pod.name().unwrap_or_default();
    let pod_ns = pod.metadata.namespace.as_deref();

    for node in hierarchy.iter_mut() {
        remove_pod_node(node, pod_name.as_ref(), pod_ns);
    }

    let pod_labels = pod.labels();
    for namespace_node in hierarchy.iter_mut() {
        if namespace_node.kind == ResourceKind::Namespace
            && namespace_node.metadata.name.as_deref() == pod_ns
        {
            let mut pod_added_to_service = false;

            add_pod_to_matching_services(
                namespace_node,
                pod,
                pod_labels,
                &mut pod_added_to_service,
            );

            if !pod_added_to_service {
                namespace_node.relatives.push(new_pod(pod));
            }
            break;
        }
    }
}

fn add_pod_to_matching_services(
    node: &mut HierarchyNode,
    pod: &Pod,
    pod_labels: &BTreeMap<String, String>,
    pod_added: &mut bool,
) {
    if node.kind == ResourceKind::Service
        && let Some(ResourceSpec::Service(service_spec)) = &node.spec
    {
        let service_ns = node.metadata.namespace.as_deref();
        let pod_ns = pod.metadata.namespace.as_deref();

        if service_ns == pod_ns
            && selectors_match(
                &service_spec.selector.clone().unwrap_or_default(),
                pod_labels,
            )
        {
            node.relatives.push(new_pod(pod));
            *pod_added = true;
        }
    }

    for child in node.relatives.iter_mut() {
        add_pod_to_matching_services(child, pod, pod_labels, pod_added);
    }
}

async fn build_initial_relationships(ctx: Context) {
    println!("Building initial relationships between services and pods...");
    let namespace_snapshot = ctx.namespace_store.state();
    let services_snapshot = ctx.service_store.state();
    let pods_snapshot = ctx.pod_store.state();
    let httproute_snapshot = ctx.httproute_store.state();

    info!(
        "Found {} namespaces, {} services, and {} pods to process",
        namespace_snapshot.len(),
        services_snapshot.len(),
        pods_snapshot.len()
    );

    let mut hierarchy = ctx.state.hierarchy.write().await;
    let mut assigned_nodes: HashSet<String> = HashSet::new();

    for namespace in namespace_snapshot.iter() {
        let metadata = namespace.metadata.clone();
        let spec = Some(ResourceSpec::Namespace(()));
        let resource_metadata =
            extract_resource_metadata(&ResourceKind::Namespace, &metadata, &spec);

        let namespace_node = HierarchyNode {
            kind: ResourceKind::Namespace,
            name: namespace.name().unwrap_or_default().to_string(),
            relatives: Vec::new(),
            metadata,
            spec,
            resource_metadata,
        };
        info!("adding namespace {:?}", namespace_node.name);
        hierarchy.push(namespace_node);
    }

    for httproute in httproute_snapshot.iter() {
        if let Some(namespace) = hierarchy.iter_mut().find(|node| {
            node.kind == ResourceKind::Namespace
                && httproute.metadata.namespace == node.metadata.name
        }) {
            let metadata = httproute.metadata.clone();
            let spec = Some(ResourceSpec::HTTPRoute(httproute.spec.clone()));
            let resource_metadata =
                extract_resource_metadata(&ResourceKind::HTTPRoute, &metadata, &spec);

            let httproute_node = HierarchyNode {
                kind: ResourceKind::HTTPRoute,
                name: httproute.name().unwrap_or_default().to_string(),
                relatives: Vec::new(),
                metadata,
                spec,
                resource_metadata,
            };

            info!(
                "adding httproute {:?} to namespace {:?}",
                httproute_node.name, namespace.name
            );
            namespace.relatives.push(httproute_node);
        }
    }

    for service in services_snapshot.iter() {
        let service_namespace = service.metadata.namespace.clone().unwrap_or_default();
        let service_spec = service.spec.clone().unwrap_or_default();

        let mut service_node = new_service(service);

        for pod in pods_snapshot.iter() {
            let pod_name = pod.name().unwrap_or_default();
            let pod_node = new_pod(pod);
            let pod_namespace = match pod.metadata.namespace.as_deref() {
                Some(ns) => ns,
                None => continue,
            };

            if pod_namespace != service_namespace {
                continue;
            }

            let matches = match (service_spec.selector.as_ref(), pod.metadata.labels.as_ref()) {
                (Some(selectors), Some(labels)) => selectors_match(selectors, labels),
                _ => false,
            };

            if matches {
                info!(
                    "adding pod {:?} to service {:?}",
                    pod_node.name, service_node.name
                );
                service_node.relatives.push(pod_node);
                assigned_nodes.insert(pod_name.as_ref().to_string());
            }
        }

        let mut service_added_to_httproute = false;
        if let Some(namespace) = hierarchy.iter_mut().find(|node| {
            node.kind == ResourceKind::Namespace && node.metadata.name == service.metadata.namespace
        }) {
            namespace.relatives.iter_mut().for_each(|node| {
                if let Some(ResourceSpec::HTTPRoute(spec)) = &node.spec {
                    spec.rules
                        .iter()
                        .flatten()
                        .flat_map(|rule| &rule.backend_refs)
                        .flatten()
                        .for_each(|r| {
                            if let Some(kind) = &r.kind
                                && kind == &ResourceKind::Service.to_string()
                                && r.name == service.metadata.name.clone().unwrap_or_default()
                            {
                                info!(
                                    "adding service {:?} to httproute {:?}",
                                    service_node.name, node.name
                                );
                                node.relatives.push(service_node.clone());
                                service_added_to_httproute = true;
                            }
                        });
                }
            });
        }

        if service_added_to_httproute {
            assigned_nodes.insert(service.name().unwrap_or_default().as_ref().to_string());
        }

        if !service_added_to_httproute
            && let Some(namespace_node) = hierarchy.iter_mut().find(|node| {
                node.kind == ResourceKind::Namespace
                    && node.metadata.name == service.metadata.namespace
            })
        {
            info!(
                "adding service {:?} to namespace {:?}",
                service_node.name, namespace_node.name
            );
            namespace_node.relatives.push(service_node);
            assigned_nodes.insert(service.name().unwrap_or_default().as_ref().to_string());
        }
    }

    for pod in pods_snapshot.iter() {
        let pod_namespace = pod.metadata.namespace.as_deref().unwrap_or_default();
        let pod_name = pod.name().unwrap_or_default();

        if assigned_nodes.contains(pod_name.as_ref()) {
            continue;
        }

        let pod_node = new_pod(pod);

        if let Some(namespace_node) = hierarchy
            .iter_mut()
            .find(|node| node.kind == ResourceKind::Namespace && node.name == pod_namespace)
        {
            info!(
                "adding pod {:?} to namespace {:?}",
                pod_name, namespace_node.name
            );
            namespace_node.relatives.push(pod_node);
        }
    }

    for service in services_snapshot.iter() {
        let service_namespace = service.metadata.namespace.as_deref().unwrap_or_default();
        let service_name = service.name().unwrap_or_default();

        if assigned_nodes.contains(service_name.as_ref()) {
            continue;
        }

        let service_node = new_service(service);

        if let Some(namespace_node) = hierarchy
            .iter_mut()
            .find(|node| node.kind == ResourceKind::Namespace && node.name == service_namespace)
        {
            info!(
                "adding service {:?} to namespace {:?}",
                service_name, namespace_node.name
            );
            namespace_node.relatives.push(service_node);
        }
    }
}

pub fn selectors_match(
    selectors: &BTreeMap<String, String>,
    labels: &BTreeMap<String, String>,
) -> bool {
    selectors
        .iter()
        .all(|(key, value)| labels.get(key) == Some(value))
}

pub async fn pod_watcher<S>(ctx: Context, mut pod_stream: S)
where
    S: Stream<Item = Result<watcher::Event<v1::Pod>, WatcherError>> + Unpin,
{
    info!("pod watcher started, waiting for events...");

    while let Some(event) = pod_stream.next().await {
        match event {
            Ok(ev) => match ev {
                watcher::Event::Apply(pod) => {
                    info!(
                        "pod applied: {}",
                        pod.metadata.name.clone().unwrap_or_default()
                    );

                    let mut hierarchy = ctx.state.hierarchy.write().await;
                    update_pod_relationships(&mut hierarchy, &pod);
                }
                watcher::Event::Delete(pod) => {
                    info!(
                        "pod deleted: {}",
                        pod.metadata.name.clone().unwrap_or_default()
                    );

                    let pod_name = pod.metadata.name.as_deref().unwrap_or_default();
                    let pod_ns = pod.metadata.namespace.as_deref();

                    let mut nodes = ctx.state.hierarchy.write().await;
                    for root in nodes.iter_mut() {
                        remove_pod_node(root, pod_name, pod_ns);
                    }
                }
                _ => {}
            },
            Err(err) => {
                error!("error from pod stream {:?}", err)
            }
        }
    }
}

pub async fn service_watcher<S>(ctx: Context, mut service_stream: S)
where
    S: Stream<Item = Result<watcher::Event<v1::Service>, WatcherError>> + Unpin,
{
    info!("service watcher started, waiting for events...");

    while let Some(event) = service_stream.next().await {
        match event {
            Ok(ev) => match ev {
                watcher::Event::Apply(service) => {
                    info!(
                        "service applied: {}",
                        service.metadata.name.clone().unwrap_or_default()
                    );

                    let pods_snapshot: Vec<Pod> = ctx
                        .pod_store
                        .state()
                        .iter()
                        .map(|pod| pod.as_ref().clone())
                        .collect();
                    let mut hierarchy = ctx.state.hierarchy.write().await;
                    update_service_relationships(&mut hierarchy, &service, &pods_snapshot);
                }
                watcher::Event::Delete(service) => {
                    info!(
                        "service deleted: {}",
                        service.metadata.name.clone().unwrap_or_default()
                    );

                    let service_name = service.metadata.name.as_deref().unwrap_or_default();
                    let service_ns = service.metadata.namespace.as_deref();

                    let mut hierarchy = ctx.state.hierarchy.write().await;
                    for node in hierarchy.iter_mut() {
                        remove_service_node(node, service_name, service_ns);
                    }
                }
                _ => {}
            },
            Err(err) => {
                error!("error from service stream {:?}", err);
            }
        }
    }
}

pub async fn namespace_watcher<S>(ctx: Context, mut namespace_stream: S)
where
    S: Stream<Item = Result<watcher::Event<Namespace>, WatcherError>> + Unpin,
{
    info!("namespace watcher started, waiting for events...");

    while let Some(event) = namespace_stream.next().await {
        match event {
            Ok(ev) => match ev {
                watcher::Event::Apply(namespace) => {
                    info!(
                        "namespace applied: {}",
                        namespace.metadata.name.clone().unwrap_or_default()
                    );

                    let namespace_name = namespace.name().unwrap_or_default();
                    let mut hierarchy = ctx.state.hierarchy.write().await;

                    if !hierarchy.iter().any(|node| {
                        node.kind == ResourceKind::Namespace && node.name == namespace_name.as_ref()
                    }) {
                        let metadata = namespace.metadata.clone();
                        let spec = Some(ResourceSpec::Namespace(()));
                        let resource_metadata =
                            extract_resource_metadata(&ResourceKind::Namespace, &metadata, &spec);

                        let namespace_node = HierarchyNode {
                            kind: ResourceKind::Namespace,
                            name: namespace_name.as_ref().to_string(),
                            relatives: Vec::new(),
                            metadata,
                            spec,
                            resource_metadata,
                        };
                        hierarchy.push(namespace_node);

                        let services_snapshot: Vec<Service> = ctx
                            .service_store
                            .state()
                            .iter()
                            .map(|service| service.as_ref().clone())
                            .collect();
                        let httproutes_snapshot: Vec<HTTPRoute> = ctx
                            .httproute_store
                            .state()
                            .iter()
                            .map(|route| route.as_ref().clone())
                            .collect();
                        let pods_snapshot: Vec<Pod> = ctx
                            .pod_store
                            .state()
                            .iter()
                            .map(|pod| pod.as_ref().clone())
                            .collect();

                        for httproute in httproutes_snapshot.iter() {
                            if httproute.metadata.namespace.as_deref()
                                == Some(namespace_name.as_ref())
                            {
                                update_httproute_relationships(
                                    &mut hierarchy,
                                    httproute,
                                    &services_snapshot,
                                    &pods_snapshot,
                                );
                            }
                        }

                        for service in services_snapshot.iter() {
                            if service.metadata.namespace.as_deref()
                                == Some(namespace_name.as_ref())
                            {
                                update_service_relationships(
                                    &mut hierarchy,
                                    service,
                                    &pods_snapshot,
                                );
                            }
                        }

                        for pod in pods_snapshot.iter() {
                            if pod.metadata.namespace.as_deref() == Some(namespace_name.as_ref()) {
                                let mut pod_assigned = false;
                                if let Some(ns_node) = hierarchy.iter().find(|node| {
                                    node.kind == ResourceKind::Namespace
                                        && node.name == namespace_name.as_ref()
                                }) {
                                    fn check_pod_in_hierarchy(
                                        node: &HierarchyNode,
                                        pod_name: &str,
                                    ) -> bool {
                                        if node.kind == ResourceKind::Pod && node.name == pod_name {
                                            return true;
                                        }
                                        node.relatives
                                            .iter()
                                            .any(|child| check_pod_in_hierarchy(child, pod_name))
                                    }
                                    pod_assigned = check_pod_in_hierarchy(
                                        ns_node,
                                        pod.name().unwrap_or_default().as_ref(),
                                    );
                                }

                                if !pod_assigned
                                    && let Some(ns_node) = hierarchy.iter_mut().find(|node| {
                                        node.kind == ResourceKind::Namespace
                                            && node.name == namespace_name.as_ref()
                                    })
                                {
                                    ns_node.relatives.push(new_pod(pod));
                                }
                            }
                        }
                    }
                }
                watcher::Event::Delete(namespace) => {
                    info!(
                        "namespace deleted: {}",
                        namespace.metadata.name.clone().unwrap_or_default()
                    );

                    let namespace_name = namespace.name().unwrap_or_default();
                    let mut hierarchy = ctx.state.hierarchy.write().await;
                    hierarchy.retain(|node| {
                        !(node.kind == ResourceKind::Namespace
                            && node.name == namespace_name.as_ref())
                    });
                }
                _ => {}
            },
            Err(err) => {
                error!("error from namespace stream {:?}", err)
            }
        }
    }
}

pub async fn httproute_watcher<S>(ctx: Context, mut httroute_stream: S)
where
    S: Stream<Item = Result<watcher::Event<HTTPRoute>, WatcherError>> + Unpin,
{
    info!("httproute watcher started, waiting for events...");

    while let Some(event) = httroute_stream.next().await {
        match event {
            Ok(ev) => match ev {
                watcher::Event::Apply(httproute) => {
                    info!(
                        "httproute applied: {}",
                        httproute.metadata.name.clone().unwrap_or_default()
                    );

                    let services_snapshot: Vec<Service> = ctx
                        .service_store
                        .state()
                        .iter()
                        .map(|service| service.as_ref().clone())
                        .collect();
                    let pods_snapshot: Vec<Pod> = ctx
                        .pod_store
                        .state()
                        .iter()
                        .map(|pod| pod.as_ref().clone())
                        .collect();
                    let mut hierarchy = ctx.state.hierarchy.write().await;
                    update_httproute_relationships(
                        &mut hierarchy,
                        &httproute,
                        &services_snapshot,
                        &pods_snapshot,
                    );
                }
                watcher::Event::Delete(httproute) => {
                    info!(
                        "httproute deleted: {}",
                        httproute.metadata.name.clone().unwrap_or_default()
                    );

                    let httproute_name = httproute.metadata.name.as_deref().unwrap_or_default();
                    let httproute_ns = httproute.metadata.namespace.as_deref();

                    let mut hierarchy = ctx.state.hierarchy.write().await;
                    for node in hierarchy.iter_mut() {
                        remove_httproute_node(node, httproute_name, httproute_ns);
                    }

                    let services_snapshot: Vec<Service> = ctx
                        .service_store
                        .state()
                        .iter()
                        .map(|service| service.as_ref().clone())
                        .collect();
                    let pods_snapshot: Vec<Pod> = ctx
                        .pod_store
                        .state()
                        .iter()
                        .map(|pod| pod.as_ref().clone())
                        .collect();

                    for service in services_snapshot.iter() {
                        if service.metadata.namespace.as_deref() == httproute_ns {
                            update_service_relationships(&mut hierarchy, service, &pods_snapshot);
                        }
                    }
                }
                _ => {}
            },
            Err(err) => {
                error!("error from httproute stream {:?}", err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::core::v1::{PodSpec, PodStatus, ServicePort, ServiceSpec};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use std::collections::BTreeMap;

    fn create_test_namespace(name: &str) -> HierarchyNode {
        HierarchyNode {
            kind: ResourceKind::Namespace,
            name: name.to_string(),
            relatives: Vec::new(),
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: None,
                ..Default::default()
            },
            spec: Some(ResourceSpec::Namespace(())),
            resource_metadata: ResourceMetadata {
                hostnames: None,
                selectors: None,
                ports: None,
                port_mappings: None,
                target_ports: None,
                target_port_names: None,
                labels: None,
                phase: None,
                backend_refs: None,
                service_type: None,
                cluster_ips: None,
                external_ips: None,
                pod_ips: None,
                container_ports: None,
            },
        }
    }

    fn create_test_pod(name: &str, namespace: &str, labels: BTreeMap<String, String>) -> Pod {
        Pod {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some(labels),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![],
                ..Default::default()
            }),
            status: Some(PodStatus {
                phase: Some("Running".to_string()),
                ..Default::default()
            }),
        }
    }

    fn create_test_service(
        name: &str,
        namespace: &str,
        selector: BTreeMap<String, String>,
    ) -> Service {
        Service {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                selector: Some(selector),
                ports: Some(vec![ServicePort {
                    port: 80,
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            status: None,
        }
    }

    fn create_test_httproute(name: &str, namespace: &str, _backend_service: &str) -> HTTPRoute {
        HTTPRoute {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            spec: HTTPRouteSpec {
                hostnames: Some(vec!["example.com".to_string()]),
                ..Default::default()
            },
            status: None,
        }
    }

    #[test]
    fn test_selectors_match() {
        let mut selectors = BTreeMap::new();
        selectors.insert("app".to_string(), "web".to_string());
        selectors.insert("version".to_string(), "v1".to_string());

        let mut matching_labels = BTreeMap::new();
        matching_labels.insert("app".to_string(), "web".to_string());
        matching_labels.insert("version".to_string(), "v1".to_string());
        matching_labels.insert("env".to_string(), "prod".to_string());

        let mut non_matching_labels = BTreeMap::new();
        non_matching_labels.insert("app".to_string(), "api".to_string());
        non_matching_labels.insert("version".to_string(), "v1".to_string());

        assert!(selectors_match(&selectors, &matching_labels));
        assert!(!selectors_match(&selectors, &non_matching_labels));
    }

    #[test]
    fn test_new_pod_creation() {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "web".to_string());

        let pod = create_test_pod("test-pod", "default", labels.clone());
        let hierarchy_node = new_pod(&pod);

        assert_eq!(hierarchy_node.kind, ResourceKind::Pod);
        assert_eq!(hierarchy_node.name, "test-pod");
        assert_eq!(
            hierarchy_node.metadata.namespace,
            Some("default".to_string())
        );
        assert_eq!(hierarchy_node.resource_metadata.labels, Some(labels));
        assert_eq!(
            hierarchy_node.resource_metadata.phase,
            Some("Running".to_string())
        );
    }

    #[test]
    fn test_new_service_creation() {
        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());

        let service = create_test_service("test-service", "default", selector.clone());
        let hierarchy_node = new_service(&service);

        assert_eq!(hierarchy_node.kind, ResourceKind::Service);
        assert_eq!(hierarchy_node.name, "test-service");
        assert_eq!(
            hierarchy_node.metadata.namespace,
            Some("default".to_string())
        );
        assert_eq!(hierarchy_node.resource_metadata.selectors, Some(selector));
        assert_eq!(hierarchy_node.resource_metadata.ports, Some(vec![80]));
    }

    #[test]
    fn test_remove_pod_node() {
        let mut namespace = create_test_namespace("default");

        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "web".to_string());
        let pod = create_test_pod("test-pod", "default", labels);
        let pod_node = new_pod(&pod);
        namespace.relatives.push(pod_node);

        assert_eq!(namespace.relatives.len(), 1);

        remove_pod_node(&mut namespace, "test-pod", Some("default"));

        assert_eq!(namespace.relatives.len(), 0);
    }

    #[test]
    fn test_remove_service_node() {
        let mut namespace = create_test_namespace("default");

        let selector = BTreeMap::new();
        let service = create_test_service("test-service", "default", selector);
        let service_node = new_service(&service);
        namespace.relatives.push(service_node);

        assert_eq!(namespace.relatives.len(), 1);

        remove_service_node(&mut namespace, "test-service", Some("default"));

        assert_eq!(namespace.relatives.len(), 0);
    }

    #[test]
    fn test_update_service_relationships_with_matching_pod() {
        let mut hierarchy = vec![create_test_namespace("default")];

        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());
        let service = create_test_service("web-service", "default", selector.clone());

        let pod = create_test_pod("web-pod", "default", selector);
        let pods = vec![pod];

        update_service_relationships(&mut hierarchy, &service, &pods);

        assert_eq!(hierarchy.len(), 1);
        assert_eq!(hierarchy[0].relatives.len(), 1);
        assert_eq!(hierarchy[0].relatives[0].kind, ResourceKind::Service);
        assert_eq!(hierarchy[0].relatives[0].name, "web-service");
        assert_eq!(hierarchy[0].relatives[0].relatives.len(), 1);
        assert_eq!(
            hierarchy[0].relatives[0].relatives[0].kind,
            ResourceKind::Pod
        );
        assert_eq!(hierarchy[0].relatives[0].relatives[0].name, "web-pod");
    }

    #[test]
    fn test_update_httproute_relationships() {
        let mut hierarchy = vec![create_test_namespace("default")];

        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());
        let service = create_test_service("web-service", "default", selector.clone());
        let services = vec![service];

        let pod = create_test_pod("web-pod", "default", selector);
        let pods = vec![pod];

        let httproute = create_test_httproute("web-route", "default", "web-service");

        update_httproute_relationships(&mut hierarchy, &httproute, &services, &pods);

        assert_eq!(hierarchy.len(), 1);
        assert_eq!(hierarchy[0].relatives.len(), 1);

        let httproute_node = &hierarchy[0].relatives[0];
        assert_eq!(httproute_node.kind, ResourceKind::HTTPRoute);
        assert_eq!(httproute_node.name, "web-route");
        assert_eq!(
            httproute_node.resource_metadata.hostnames,
            Some(vec!["example.com".to_string()])
        );
    }

    #[test]
    fn test_service_relationships_without_httproute() {
        let mut hierarchy = vec![create_test_namespace("default")];

        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "api".to_string());
        let service = create_test_service("api-service", "default", selector.clone());

        let pod = create_test_pod("api-pod", "default", selector);
        let pods = vec![pod];

        update_service_relationships(&mut hierarchy, &service, &pods);

        assert_eq!(hierarchy.len(), 1);
        assert_eq!(hierarchy[0].relatives.len(), 1);
        assert_eq!(hierarchy[0].relatives[0].kind, ResourceKind::Service);
        assert_eq!(hierarchy[0].relatives[0].name, "api-service");
        assert_eq!(hierarchy[0].relatives[0].relatives.len(), 1);
        assert_eq!(
            hierarchy[0].relatives[0].relatives[0].kind,
            ResourceKind::Pod
        );
        assert_eq!(hierarchy[0].relatives[0].relatives[0].name, "api-pod");
    }

    #[test]
    fn test_extract_resource_metadata_httproute() {
        let httproute = create_test_httproute("test-route", "default", "test-service");
        let metadata = httproute.metadata.clone();
        let spec = Some(ResourceSpec::HTTPRoute(httproute.spec.clone()));

        let resource_metadata =
            extract_resource_metadata(&ResourceKind::HTTPRoute, &metadata, &spec);

        assert_eq!(
            resource_metadata.hostnames,
            Some(vec!["example.com".to_string()])
        );
        assert!(resource_metadata.selectors.is_none());
        assert!(resource_metadata.ports.is_none());
    }

    #[test]
    fn test_extract_resource_metadata_service() {
        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());
        let service = create_test_service("test-service", "default", selector.clone());
        let metadata = service.metadata.clone();
        let spec = Some(ResourceSpec::Service(Box::new(
            service.spec.clone().unwrap(),
        )));

        let resource_metadata = extract_resource_metadata(&ResourceKind::Service, &metadata, &spec);

        assert_eq!(resource_metadata.selectors, Some(selector));
        assert_eq!(resource_metadata.ports, Some(vec![80]));
        assert!(resource_metadata.hostnames.is_none());
        assert!(resource_metadata.backend_refs.is_none());
    }

    #[test]
    fn test_hierarchy_state_after_multiple_events() {
        let mut hierarchy = vec![create_test_namespace("default")];

        let httproute = create_test_httproute("web-route", "default", "web-service");
        let services = vec![];
        let pods = vec![];
        update_httproute_relationships(&mut hierarchy, &httproute, &services, &pods);

        assert_eq!(hierarchy[0].relatives.len(), 1);
        assert_eq!(hierarchy[0].relatives[0].kind, ResourceKind::HTTPRoute);

        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());
        let service = create_test_service("web-service", "default", selector.clone());
        update_service_relationships(&mut hierarchy, &service, &pods);

        assert_eq!(hierarchy[0].relatives.len(), 2);
        let httproute_node = &hierarchy[0].relatives[0];
        assert_eq!(httproute_node.kind, ResourceKind::HTTPRoute);
        let service_node = &hierarchy[0].relatives[1];
        assert_eq!(service_node.kind, ResourceKind::Service);

        let pod = create_test_pod("web-pod", "default", selector);
        let pods = vec![pod];
        update_service_relationships(&mut hierarchy, &service, &pods);

        let service_node = &hierarchy[0].relatives[1];
        assert_eq!(service_node.relatives.len(), 1);
        assert_eq!(service_node.relatives[0].kind, ResourceKind::Pod);
        assert_eq!(service_node.relatives[0].name, "web-pod");
    }

    #[test]
    fn test_remove_httproute_node() {
        let mut namespace = create_test_namespace("default");

        let httproute = create_test_httproute("test-route", "default", "test-service");
        let metadata = httproute.metadata.clone();
        let spec = Some(ResourceSpec::HTTPRoute(httproute.spec.clone()));
        let resource_metadata =
            extract_resource_metadata(&ResourceKind::HTTPRoute, &metadata, &spec);

        let httproute_node = HierarchyNode {
            kind: ResourceKind::HTTPRoute,
            name: "test-route".to_string(),
            relatives: Vec::new(),
            metadata,
            spec,
            resource_metadata,
        };
        namespace.relatives.push(httproute_node);

        assert_eq!(namespace.relatives.len(), 1);

        remove_httproute_node(&mut namespace, "test-route", Some("default"));

        assert_eq!(namespace.relatives.len(), 0);
    }

    #[test]
    fn test_extract_port_info_numeric_ports() {
        use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

        let ports = vec![
            ServicePort {
                port: 80,
                target_port: Some(IntOrString::Int(8080)),
                ..Default::default()
            },
            ServicePort {
                port: 443,
                target_port: Some(IntOrString::Int(443)),
                ..Default::default()
            },
            ServicePort {
                port: 9090,
                target_port: None,
                ..Default::default()
            },
        ];

        let port_info = extract_port_info(&ports);

        assert_eq!(port_info.service_ports, vec![80, 443, 9090]);
        assert_eq!(port_info.port_mappings, vec!["80→8080", "443", "9090"]);
        assert_eq!(port_info.target_ports, vec![8080, 443, 9090]);
        assert_eq!(port_info.target_port_names, Vec::<String>::new());
    }

    #[test]
    fn test_extract_port_info_named_ports() {
        use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

        let ports = vec![
            ServicePort {
                port: 80,
                target_port: Some(IntOrString::String("http".to_string())),
                ..Default::default()
            },
            ServicePort {
                port: 443,
                target_port: Some(IntOrString::String("https".to_string())),
                ..Default::default()
            },
        ];

        let port_info = extract_port_info(&ports);

        assert_eq!(port_info.service_ports, vec![80, 443]);
        assert_eq!(port_info.port_mappings, vec!["80→http", "443→https"]);
        assert_eq!(port_info.target_ports, vec![80, 443]);
        assert_eq!(port_info.target_port_names, vec!["http", "https"]);
    }

    #[test]
    fn test_extract_port_info_named_port_with_numeric_target() {
        use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

        let ports = vec![ServicePort {
            name: Some("http".to_string()),
            port: 80,
            target_port: Some(IntOrString::Int(8989)),
            ..Default::default()
        }];

        let port_info = extract_port_info(&ports);

        assert_eq!(port_info.service_ports, vec![80]);
        assert_eq!(port_info.port_mappings, vec!["80→8989"]);
        assert_eq!(port_info.target_ports, vec![8989]);
        assert_eq!(port_info.target_port_names, Vec::<String>::new());
    }

    #[test]
    fn test_extract_port_info_mixed_ports() {
        use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

        let ports = vec![
            ServicePort {
                port: 80,
                target_port: Some(IntOrString::String("http".to_string())),
                ..Default::default()
            },
            ServicePort {
                port: 443,
                target_port: Some(IntOrString::Int(8443)),
                ..Default::default()
            },
            ServicePort {
                port: 3000,
                target_port: Some(IntOrString::Int(3000)),
                ..Default::default()
            },
            ServicePort {
                port: 9000,
                target_port: None,
                ..Default::default()
            },
        ];

        let port_info = extract_port_info(&ports);

        assert_eq!(port_info.service_ports, vec![80, 443, 3000, 9000]);
        assert_eq!(
            port_info.port_mappings,
            vec!["80→http", "443→8443", "3000", "9000"]
        );
        assert_eq!(port_info.target_ports, vec![80, 8443, 3000, 9000]);
        assert_eq!(port_info.target_port_names, vec!["http"]);
    }

    #[test]
    fn test_extract_resource_metadata_service_with_ports() {
        use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

        let metadata = ObjectMeta {
            name: Some("test-service".to_string()),
            namespace: Some("default".to_string()),
            ..Default::default()
        };

        let service_spec = ServiceSpec {
            selector: Some({
                let mut selector = BTreeMap::new();
                selector.insert("app".to_string(), "web".to_string());
                selector
            }),
            ports: Some(vec![
                ServicePort {
                    port: 80,
                    target_port: Some(IntOrString::String("http".to_string())),
                    ..Default::default()
                },
                ServicePort {
                    port: 443,
                    target_port: Some(IntOrString::Int(8443)),
                    ..Default::default()
                },
            ]),
            type_: Some("ClusterIP".to_string()),
            cluster_ip: Some("10.1.2.3".to_string()),
            ..Default::default()
        };

        let spec = Some(ResourceSpec::Service(Box::new(service_spec)));
        let resource_metadata = extract_resource_metadata(&ResourceKind::Service, &metadata, &spec);

        assert_eq!(resource_metadata.ports, Some(vec![80, 443]));
        assert_eq!(
            resource_metadata.port_mappings,
            Some(vec!["80→http".to_string(), "443→8443".to_string()])
        );
        assert_eq!(resource_metadata.target_ports, Some(vec![80, 8443]));
        assert_eq!(
            resource_metadata.target_port_names,
            Some(vec!["http".to_string()])
        );
        assert_eq!(
            resource_metadata.service_type,
            Some("ClusterIP".to_string())
        );
        assert_eq!(
            resource_metadata.cluster_ips,
            Some(vec!["10.1.2.3".to_string()])
        );
    }

    #[test]
    fn test_extract_container_ports_from_pod() {
        use k8s_openapi::api::core::v1::{Container, ContainerPort};

        let metadata = ObjectMeta {
            name: Some("test-pod".to_string()),
            namespace: Some("default".to_string()),
            labels: Some({
                let mut labels = BTreeMap::new();
                labels.insert("app".to_string(), "web".to_string());
                labels
            }),
            ..Default::default()
        };

        let pod_spec = PodSpec {
            containers: vec![Container {
                name: "web".to_string(),
                ports: Some(vec![
                    ContainerPort {
                        container_port: 8080,
                        name: Some("http".to_string()),
                        protocol: Some("TCP".to_string()),
                        ..Default::default()
                    },
                    ContainerPort {
                        container_port: 8443,
                        name: Some("https".to_string()),
                        protocol: Some("TCP".to_string()),
                        ..Default::default()
                    },
                    ContainerPort {
                        container_port: 9090,
                        name: None,
                        protocol: Some("TCP".to_string()),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }],
            ..Default::default()
        };

        let spec = Some(ResourceSpec::Pod(Box::new(pod_spec)));
        let resource_metadata = extract_resource_metadata(&ResourceKind::Pod, &metadata, &spec);

        let container_ports = resource_metadata.container_ports.unwrap();
        assert_eq!(container_ports.len(), 3);

        assert_eq!(container_ports[0].port, 8080);
        assert_eq!(container_ports[0].name, Some("http".to_string()));
        assert_eq!(container_ports[0].protocol, Some("TCP".to_string()));

        assert_eq!(container_ports[1].port, 8443);
        assert_eq!(container_ports[1].name, Some("https".to_string()));
        assert_eq!(container_ports[1].protocol, Some("TCP".to_string()));

        assert_eq!(container_ports[2].port, 9090);
        assert_eq!(container_ports[2].name, None);
        assert_eq!(container_ports[2].protocol, Some("TCP".to_string()));
    }

    #[test]
    fn test_extract_container_ports_multiple_containers() {
        use k8s_openapi::api::core::v1::{Container, ContainerPort};

        let metadata = ObjectMeta {
            name: Some("test-pod".to_string()),
            namespace: Some("default".to_string()),
            ..Default::default()
        };

        let pod_spec = PodSpec {
            containers: vec![
                Container {
                    name: "web".to_string(),
                    ports: Some(vec![ContainerPort {
                        container_port: 8080,
                        name: Some("http".to_string()),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
                Container {
                    name: "sidecar".to_string(),
                    ports: Some(vec![ContainerPort {
                        container_port: 9000,
                        name: Some("metrics".to_string()),
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let spec = Some(ResourceSpec::Pod(Box::new(pod_spec)));
        let resource_metadata = extract_resource_metadata(&ResourceKind::Pod, &metadata, &spec);

        let container_ports = resource_metadata.container_ports.unwrap();
        assert_eq!(container_ports.len(), 2);

        assert_eq!(container_ports[0].port, 8080);
        assert_eq!(container_ports[0].name, Some("http".to_string()));

        assert_eq!(container_ports[1].port, 9000);
        assert_eq!(container_ports[1].name, Some("metrics".to_string()));
    }

    #[test]
    fn test_extract_port_info_empty_ports() {
        let ports = vec![];
        let port_info = extract_port_info(&ports);

        assert_eq!(port_info.service_ports, Vec::<u32>::new());
        assert_eq!(port_info.port_mappings, Vec::<String>::new());
        assert_eq!(port_info.target_ports, Vec::<u32>::new());
        assert_eq!(port_info.target_port_names, Vec::<String>::new());
    }
}
