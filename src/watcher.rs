use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1;
use kube::{
    Client, ResourceExt,
    api::Api,
    error,
    runtime::{
        WatchStreamExt,
        reflector::{self, Lookup, Store},
        watcher::{self, Error as WatcherError},
    },
};

use std::{collections::BTreeMap, collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

use futures::FutureExt;
use gateway_api::httproutes::{HTTPRoute, HTTPRouteSpec};
use k8s_openapi::api::core::v1::{Namespace, Pod, Service};
use kube::api::ObjectMeta;
use serde;
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
pub struct HierarchyNode {
    pub kind: ResourceKind,
    pub name: String,
    pub relatives: Vec<HierarchyNode>,
    #[serde(skip)]
    pub metadata: ObjectMeta,
    #[serde(skip)]
    pub spec: Option<ResourceSpec>,
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
    namespace_store: Store<v1::Namespace>,
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

    let namespace_api: Api<v1::Namespace> = Api::all(client.clone());
    let (namespace_store, namespace_writer) = reflector::store::<v1::Namespace>();
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
        state: state,
        pod_store: pod_store,
        service_store: service_store,
        namespace_store: namespace_store,
        httproute_store: httproute_store,
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

fn new_pod(pod: &Pod) -> HierarchyNode {
    HierarchyNode {
        kind: ResourceKind::Pod,
        name: pod.metadata.name.clone().unwrap_or_default(),
        relatives: Vec::new(),
        metadata: pod.metadata.clone(),
        spec: pod.spec.clone().map(ResourceSpec::PodSpec),
    }
}

fn new_service(service: &Service) -> HierarchyNode {
    HierarchyNode {
        kind: ResourceKind::Service,
        name: service.metadata.name.clone().unwrap_or_default(),
        relatives: Vec::new(),
        metadata: service.metadata.clone(),
        spec: service.spec.clone().map(ResourceSpec::ServiceSpec),
    }
}

fn add_service(
    node: &mut HierarchyNode,
    service_node: &HierarchyNode,
    service_name: &str,
    service_ns: Option<&str>,
    pods: &[v1::Pod],
) {
    match node.kind {
        ResourceKind::HTTPRoute => {
            if let Some(ResourceSpec::HTTPRouteSpec(spec)) = &node.spec {
                let referenced = spec
                    .rules
                    .iter()
                    .flatten()
                    .flat_map(|rule| &rule.backend_refs)
                    .flatten()
                    .any(|r| {
                        r.kind.as_deref() == Some(&ResourceKind::Service.to_string())
                            && r.name == service_name
                    });

                if referenced {
                    if !node.relatives.iter().any(|s| s.name == service_name) {
                        let mut new_service = service_node.clone();

                        // Attach matching pods
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

                        node.relatives.push(new_service);
                    }
                }
            }
        }
        ResourceKind::Namespace => {
            if !node
                .relatives
                .iter()
                .any(|r| r.name == service_name && r.kind == ResourceKind::Service)
            {
                let mut new_service = service_node.clone();

                // Attach matching pods
                if let Some(ResourceSpec::ServiceSpec(service_spec)) = &service_node.spec {
                    new_service.relatives.extend(
                        pods.iter()
                            .filter(|pod| pod.metadata.namespace.as_deref() == service_ns)
                            .filter(|pod| {
                                selectors_match(
                                    &service_spec.selector.clone().unwrap_or_default(),
                                    pod.labels(),
                                )
                            })
                            .map(new_pod),
                    );
                }

                node.relatives.push(new_service);
            }
        }
        _ => {}
    }

    for child in node.relatives.iter_mut() {
        add_service(child, service_node, service_name, service_ns, pods);
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

fn add_pod(node: &mut HierarchyNode, pod: &v1::Pod, service_name: &str, service_ns: Option<&str>) {
    if node.kind == ResourceKind::Service
        && node.name == service_name
        && node.metadata.namespace.as_deref() == service_ns
    {
        if !node.relatives.iter().any(|p| {
            p.kind == ResourceKind::Pod
                && p.name == pod.metadata.name.as_deref().unwrap_or_default()
                && p.metadata.namespace.as_deref() == pod.metadata.namespace.as_deref()
        }) {
            node.relatives.push(new_pod(pod));
        }
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
        let namespace_node = HierarchyNode {
            kind: ResourceKind::Namespace,
            name: namespace.name().unwrap_or_default().to_string(),
            relatives: Vec::new(),
            metadata: namespace.metadata.clone(),
            spec: namespace.spec.clone().map(ResourceSpec::NamespaceSpec),
        };
        info!("adding namespace {:?}", namespace_node.name);
        hierarchy.push(namespace_node);
    }

    for httproute in httproute_snapshot.iter() {
        if let Some(namespace) = hierarchy.iter_mut().find(|node| {
            node.kind == ResourceKind::Namespace
                && httproute.metadata.namespace == node.metadata.name
        }) {
            let httproute_node = HierarchyNode {
                kind: ResourceKind::HTTPRoute,
                name: httproute.name().unwrap_or_default().to_string(),
                relatives: Vec::new(),
                metadata: httproute.metadata.clone(),
                spec: Some(ResourceSpec::HTTPRouteSpec(httproute.spec.clone())),
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
        let mut service_node = HierarchyNode {
            kind: ResourceKind::Service,
            name: service.name().unwrap_or_default().to_string(),
            relatives: Vec::new(),
            metadata: service.metadata.clone(),
            spec: service.spec.clone().map(ResourceSpec::ServiceSpec),
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
                            if let Some(kind) = &r.kind {
                                if kind == &ResourceKind::Service.to_string()
                                    && r.name
                                        == service
                                            .metadata
                                            .name
                                            .clone()
                                            .unwrap_or_default()
                                            .to_string()
                                {
                                    info!(
                                        "adding service {:?} to httproute {:?}",
                                        service_node.name, node.name
                                    );
                                    node.relatives.push(service_node.clone());
                                    assigned_nodes.insert(
                                        service.name().unwrap_or_default().as_ref().to_string(),
                                    );
                                }
                            }
                        });
                }
            });
        }
    }

    for pod in pods_snapshot.iter() {
        let pod_namespace = pod.metadata.namespace.as_deref().unwrap_or_default();
        let pod_name = pod.name().unwrap_or_default();

        if assigned_nodes.contains(pod_name.as_ref()) {
            continue;
        }

        let pod_node = new_pod(&pod);

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
    selectors.iter().all(|(key, value)| {
        labels.get(key).map_or(false, |v| v == value)
    })
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
                watcher::Event::Apply(service) => {}
                watcher::Event::Delete(service) => {}
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
    S: Stream<Item = Result<watcher::Event<v1::Namespace>, WatcherError>> + Unpin,
{
    info!("namespace watcher started, waiting for events...");

    while let Some(event) = namespace_stream.next().await {
        match event {
            Ok(ev) => {}
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
            Ok(ev) => {}
            Err(err) => {
                error!("error from httproute stream {:?}", err)
            }
        }
    }
}
