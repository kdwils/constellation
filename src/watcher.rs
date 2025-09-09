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
use k8s_openapi::api::core::v1::{Namespace, Pod, Service};
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

impl ToString for ResourceKind {
    fn to_string(&self) -> String {
        match self {
            ResourceKind::HTTPRoute => "HTTPRoute".to_string(),
            ResourceKind::Service => "Service".to_string(),
            ResourceKind::Pod => "Pod".to_string(),
            ResourceKind::Namespace => "Namespace".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ResourceSpec {
    NamespaceSpec(v1::NamespaceSpec),
    ServiceSpec(v1::ServiceSpec),
    PodSpec(v1::PodSpec),
    HTTPRouteSpec(HTTPRouteSpec),
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
    pub labels: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_refs: Option<Vec<String>>,
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
                Some(ResourceSpec::HTTPRouteSpec(spec)) => {
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
                labels: None,
                phase: None,
                backend_refs,
            }
        }
        ResourceKind::Service => {
            let (selectors, ports) = match spec {
                Some(ResourceSpec::ServiceSpec(spec)) => {
                    let selectors = spec.selector.clone();
                    let ports = spec.ports.as_ref().map(|port_list| {
                        port_list.iter().map(|p| p.port as u32).collect::<Vec<_>>()
                    });
                    (selectors, ports)
                }
                _ => (None, None),
            };
            ResourceMetadata {
                hostnames: None,
                selectors,
                ports,
                labels: None,
                phase: None,
                backend_refs: None,
            }
        }
        ResourceKind::Pod => {
            let labels = metadata.labels.clone();
            let ports = match spec {
                Some(ResourceSpec::PodSpec(spec)) => {
                    let mut port_list = Vec::new();
                    for container in &spec.containers {
                        if let Some(container_ports) = &container.ports {
                            for port in container_ports {
                                port_list.push(port.container_port as u32);
                            }
                        }
                    }
                    if port_list.is_empty() {
                        None
                    } else {
                        Some(port_list)
                    }
                }
                _ => None,
            };
            ResourceMetadata {
                hostnames: None,
                selectors: None,
                ports,
                labels,
                phase: None,
                backend_refs: None,
            }
        }
        ResourceKind::Namespace => ResourceMetadata {
            hostnames: None,
            selectors: None,
            ports: None,
            labels: None,
            phase: None,
            backend_refs: None,
        },
    }
}

fn new_pod(pod: &Pod) -> HierarchyNode {
    let spec = pod.spec.clone().map(ResourceSpec::PodSpec);
    let metadata = pod.metadata.clone();
    let mut resource_metadata = extract_resource_metadata(&ResourceKind::Pod, &metadata, &spec);

    // Extract phase from pod status
    resource_metadata.phase = pod.status.as_ref().and_then(|status| status.phase.clone());

    HierarchyNode {
        kind: ResourceKind::Pod,
        name: pod.metadata.name.clone().unwrap_or_default(),
        relatives: Vec::new(),
        metadata,
        spec,
        resource_metadata,
    }
}

fn new_service(service: &Service) -> HierarchyNode {
    let spec = service.spec.clone().map(ResourceSpec::ServiceSpec);
    let metadata = service.metadata.clone();
    let resource_metadata = extract_resource_metadata(&ResourceKind::Service, &metadata, &spec);

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

fn update_service_relationships(
    hierarchy: &mut Vec<HierarchyNode>,
    service: &Service,
    pods: &[Pod],
) {
    let service_name = service.name().unwrap_or_default();
    let service_ns = service.metadata.namespace.as_deref();
    let service_node = new_service(service);

    // Remove existing service if present
    for node in hierarchy.iter_mut() {
        remove_service_node(node, service_name.as_ref(), service_ns);
    }

    // Add service with current pod relationships
    for namespace_node in hierarchy.iter_mut() {
        if namespace_node.kind == ResourceKind::Namespace
            && namespace_node.metadata.name.as_deref() == service_ns
        {
            // Check if service should be added to any HTTPRoutes in this namespace
            let mut service_added_to_httproute = false;

            for httproute in namespace_node.relatives.iter_mut() {
                if httproute.kind == ResourceKind::HTTPRoute
                    && let Some(ResourceSpec::HTTPRouteSpec(spec)) = &httproute.spec
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

                        // Add matching pods to the service
                        if let Some(ResourceSpec::ServiceSpec(service_spec)) = &service_node.spec {
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

            // If service wasn't added to any HTTPRoute, add it directly to namespace
            if !service_added_to_httproute {
                let mut new_service = service_node.clone();

                // Add matching pods to the service
                if let Some(ResourceSpec::ServiceSpec(service_spec)) = &service_node.spec {
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
    hierarchy: &mut Vec<HierarchyNode>,
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
            let spec = Some(ResourceSpec::HTTPRouteSpec(httproute.spec.clone()));
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

            // Add referenced services to the HTTPRoute
            if let Some(ResourceSpec::HTTPRouteSpec(spec)) = &httproute_node.spec {
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

                        // Add matching pods to the service
                        if let Some(ResourceSpec::ServiceSpec(service_spec)) = &service_node.spec {
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

fn add_pod(node: &mut HierarchyNode, pod: &v1::Pod, service_name: &str, service_ns: Option<&str>) {
    if node.kind == ResourceKind::Service
        && node.name == service_name
        && node.metadata.namespace.as_deref() == service_ns
        && !node.relatives.iter().any(|p| {
            p.kind == ResourceKind::Pod
                && p.name == pod.metadata.name.as_deref().unwrap_or_default()
                && p.metadata.namespace.as_deref() == pod.metadata.namespace.as_deref()
        })
    {
        node.relatives.push(new_pod(pod));
    }

    for child in node.relatives.iter_mut() {
        add_pod(child, pod, service_name, service_ns);
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
        let spec = namespace.spec.clone().map(ResourceSpec::NamespaceSpec);
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
            let spec = Some(ResourceSpec::HTTPRouteSpec(httproute.spec.clone()));
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
        let metadata = service.metadata.clone();
        let spec = service.spec.clone().map(ResourceSpec::ServiceSpec);
        let resource_metadata = extract_resource_metadata(&ResourceKind::Service, &metadata, &spec);

        let mut service_node = HierarchyNode {
            kind: ResourceKind::Service,
            name: service.name().unwrap_or_default().to_string(),
            relatives: Vec::new(),
            metadata,
            spec,
            resource_metadata,
        };

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
                if let Some(ResourceSpec::HTTPRouteSpec(spec)) = &node.spec {
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

        // If service was added to an HTTPRoute, mark it as assigned
        if service_added_to_httproute {
            assigned_nodes.insert(service.name().unwrap_or_default().as_ref().to_string());
        }

        // If service was not added to any HTTPRoute, add it directly to namespace
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

                    let pod_ns = pod.metadata.namespace.as_deref();
                    for service in ctx.service_store.state().iter() {
                        if let Some(spec) = &service.spec {
                            let service_ns = service.metadata.namespace.as_deref();
                            if service_ns != pod_ns {
                                continue;
                            }

                            if !selectors_match(
                                &spec.selector.clone().unwrap_or_default(),
                                pod.labels(),
                            ) {
                                continue;
                            }

                            let service_name = service.metadata.name.as_deref().unwrap_or_default();

                            let mut nodes = ctx.state.hierarchy.write().await;
                            for root in nodes.iter_mut() {
                                add_pod(root, &pod, service_name, service_ns);
                            }
                        }
                    }
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

                    // Check if namespace already exists
                    if !hierarchy.iter().any(|node| {
                        node.kind == ResourceKind::Namespace && node.name == namespace_name.as_ref()
                    }) {
                        let metadata = namespace.metadata.clone();
                        let spec = namespace.spec.clone().map(ResourceSpec::NamespaceSpec);
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

                        // Add existing resources to the new namespace
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

                        // Add HTTPRoutes to this namespace
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

                        // Add services to this namespace (those not already in HTTPRoutes)
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

                        // Add orphaned pods to this namespace
                        for pod in pods_snapshot.iter() {
                            if pod.metadata.namespace.as_deref() == Some(namespace_name.as_ref()) {
                                // Check if pod is already assigned to a service
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
            spec: Some(ResourceSpec::NamespaceSpec(Default::default())),
            resource_metadata: ResourceMetadata {
                hostnames: None,
                selectors: None,
                ports: None,
                labels: None,
                phase: None,
                backend_refs: None,
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

        // Add a pod to the namespace
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "web".to_string());
        let pod = create_test_pod("test-pod", "default", labels);
        let pod_node = new_pod(&pod);
        namespace.relatives.push(pod_node);

        assert_eq!(namespace.relatives.len(), 1);

        // Remove the pod
        remove_pod_node(&mut namespace, "test-pod", Some("default"));

        assert_eq!(namespace.relatives.len(), 0);
    }

    #[test]
    fn test_remove_service_node() {
        let mut namespace = create_test_namespace("default");

        // Add a service to the namespace
        let selector = BTreeMap::new();
        let service = create_test_service("test-service", "default", selector);
        let service_node = new_service(&service);
        namespace.relatives.push(service_node);

        assert_eq!(namespace.relatives.len(), 1);

        // Remove the service
        remove_service_node(&mut namespace, "test-service", Some("default"));

        assert_eq!(namespace.relatives.len(), 0);
    }

    #[test]
    fn test_update_service_relationships_with_matching_pod() {
        let mut hierarchy = vec![create_test_namespace("default")];

        // Create a service with selector
        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());
        let service = create_test_service("web-service", "default", selector.clone());

        // Create a matching pod
        let pod = create_test_pod("web-pod", "default", selector);
        let pods = vec![pod];

        update_service_relationships(&mut hierarchy, &service, &pods);

        // Verify service was added to namespace with the pod
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

        // Create a service
        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());
        let service = create_test_service("web-service", "default", selector.clone());
        let services = vec![service];

        // Create a matching pod
        let pod = create_test_pod("web-pod", "default", selector);
        let pods = vec![pod];

        // Create an HTTPRoute
        let httproute = create_test_httproute("web-route", "default", "web-service");

        update_httproute_relationships(&mut hierarchy, &httproute, &services, &pods);

        // Verify HTTPRoute was added to namespace
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

        // Create a service
        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "api".to_string());
        let service = create_test_service("api-service", "default", selector.clone());

        // Create a matching pod
        let pod = create_test_pod("api-pod", "default", selector);
        let pods = vec![pod];

        update_service_relationships(&mut hierarchy, &service, &pods);

        // Verify service was added directly to namespace (no HTTPRoute)
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
        let spec = Some(ResourceSpec::HTTPRouteSpec(httproute.spec.clone()));

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
        let spec = Some(ResourceSpec::ServiceSpec(service.spec.clone().unwrap()));

        let resource_metadata = extract_resource_metadata(&ResourceKind::Service, &metadata, &spec);

        assert_eq!(resource_metadata.selectors, Some(selector));
        assert_eq!(resource_metadata.ports, Some(vec![80]));
        assert!(resource_metadata.hostnames.is_none());
        assert!(resource_metadata.backend_refs.is_none());
    }

    #[test]
    fn test_hierarchy_state_after_multiple_events() {
        let mut hierarchy = vec![create_test_namespace("default")];

        // Event 1: Add HTTPRoute
        let httproute = create_test_httproute("web-route", "default", "web-service");
        let services = vec![];
        let pods = vec![];
        update_httproute_relationships(&mut hierarchy, &httproute, &services, &pods);

        // Verify HTTPRoute added
        assert_eq!(hierarchy[0].relatives.len(), 1);
        assert_eq!(hierarchy[0].relatives[0].kind, ResourceKind::HTTPRoute);

        // Event 2: Add Service that matches HTTPRoute
        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());
        let service = create_test_service("web-service", "default", selector.clone());
        update_service_relationships(&mut hierarchy, &service, &pods);

        // Verify service was added directly to namespace (since our simplified HTTPRoute doesn't reference it)
        assert_eq!(hierarchy[0].relatives.len(), 2); // HTTPRoute and Service
        let httproute_node = &hierarchy[0].relatives[0];
        assert_eq!(httproute_node.kind, ResourceKind::HTTPRoute);
        let service_node = &hierarchy[0].relatives[1];
        assert_eq!(service_node.kind, ResourceKind::Service);

        // Event 3: Add Pod that matches Service
        let pod = create_test_pod("web-pod", "default", selector);
        let pods = vec![pod];
        update_service_relationships(&mut hierarchy, &service, &pods);

        // Verify pod was added to service
        let service_node = &hierarchy[0].relatives[1];
        assert_eq!(service_node.relatives.len(), 1); // Pod under Service
        assert_eq!(service_node.relatives[0].kind, ResourceKind::Pod);
        assert_eq!(service_node.relatives[0].name, "web-pod");
    }

    #[test]
    fn test_remove_httproute_node() {
        let mut namespace = create_test_namespace("default");

        // Add an HTTPRoute to the namespace
        let httproute = create_test_httproute("test-route", "default", "test-service");
        let metadata = httproute.metadata.clone();
        let spec = Some(ResourceSpec::HTTPRouteSpec(httproute.spec.clone()));
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

        // Remove the HTTPRoute
        remove_httproute_node(&mut namespace, "test-route", Some("default"));

        assert_eq!(namespace.relatives.len(), 0);
    }
}
