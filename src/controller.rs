#[derive(Debug, Serialize)]
pub struct NamespaceHierarchy {
    pub namespace: String,
    pub children: Vec<ServiceChild>,
}

#[derive(Debug, Serialize)]
pub struct ServiceChild {
    pub kind: String, // always "service"
    pub name: String,
    pub children: Vec<PodChild>,
}

#[derive(Debug, Serialize)]
pub struct PodChild {
    pub kind: String, // always "pod"
    pub name: String,
}
// ...existing code...
use futures::{FutureExt, StreamExt};
use k8s_openapi::api::core::v1::{self, Namespace, Pod, Service};
use kube::{
    Client,
    api::{Api, ObjectMeta},
    runtime::{Controller, controller::Action, watcher::Config},
};
use serde::Serialize;
use std::{collections::BTreeMap, collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct State {
    pub graph: Arc<RwLock<Graph>>,
}

impl State {
    pub fn default() -> Self {
        Self {
            graph: Arc::new(RwLock::new(Graph::default())),
        }
    }
    fn to_context(&self, client: Client) -> Arc<Context> {
        Arc::new(Context {
            state: self.clone(),
            client: client,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum ResourceKind {
    Namespace,
    Pod,
    Service,
    Ingress,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Spec {
    PodSpec(v1::PodSpec),
    ServiceSpec(v1::ServiceSpec),
    NamespaceSpec(v1::NamespaceSpec),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct NodeId {
    pub namespace: Option<String>,
    pub name: String,
}

impl NodeId {
    pub fn key(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}/{}", ns, self.name),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: ResourceKind,
    pub metadata: ObjectMeta,
    pub spec: Spec,
}

impl Node {
    pub fn new(
        kind: ResourceKind,
        name: impl Into<String>,
        namespace: impl Into<String>,
        metadata: ObjectMeta,
        spec: Spec,
    ) -> Self {
        Self {
            id: NodeId {
                namespace: Some(namespace.into()),
                name: name.into(),
            },
            kind: kind,
            metadata: metadata,
            spec: spec,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Edginfo {
    pub id: String,
    pub kind: ResourceKind,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Graph {
    pub nodes: HashMap<String, Node>,
    pub edges: HashMap<String, Vec<Edginfo>>, // parent -> [Edginfo]
    pub reverse: HashMap<String, String>,     // child -> parent
}

#[derive(Debug, Serialize)]
pub struct ResourceLink {
    pub kind: ResourceKind,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ResourceChain {
    pub chain: Vec<ResourceLink>,
}

impl Graph {
    pub fn resource_chains(&self) -> Vec<ResourceChain> {
        let parent_keys: std::collections::HashSet<_> = self.edges.keys().cloned().collect();
        self.nodes
            .values()
            .filter(|node| !parent_keys.contains(&node.id.key()))
            .map(|leaf_node| {
                let mut chain = Vec::new();
                let mut current_key = leaf_node.id.key();
                let mut visited = std::collections::HashSet::new();
                if let Some(node) = self.nodes.get(&current_key) {
                    chain.push(ResourceLink {
                        kind: node.kind.clone(),
                        name: node.id.name.clone(),
                    });
                }
                while let Some(parent_key) = self.reverse.get(&current_key) {
                    if !visited.insert(parent_key.clone()) {
                        break;
                    }
                    if let Some(parent_node) = self.nodes.get(parent_key) {
                        chain.push(ResourceLink {
                            kind: parent_node.kind.clone(),
                            name: parent_node.id.name.clone(),
                        });
                    }
                    current_key = parent_key.clone();
                }
                chain.reverse();
                ResourceChain { chain }
            })
            .collect()
    }
    /// Returns a vector of namespaces, each with its children (services, pods) in a flat, readable format.
    pub fn namespace_hierarchy(&self) -> Vec<NamespaceHierarchy> {
        let mut result = Vec::new();
        for ns_node in self
            .nodes
            .values()
            .filter(|n| n.kind == ResourceKind::Namespace)
        {
            let ns_name = &ns_node.id.name;
            // Collect all services in this namespace
            let services: Vec<ServiceChild> = self
                .nodes
                .values()
                .filter(|n| {
                    n.kind == ResourceKind::Service && n.id.namespace.as_deref() == Some(ns_name)
                })
                .map(|svc| {
                    // Collect pods attached to this service
                    let pods: Vec<PodChild> = self
                        .edges
                        .get(&svc.id.key())
                        .map(|edges| {
                            edges
                                .iter()
                                .filter_map(|e| self.nodes.get(&e.id))
                                .filter(|n| n.kind == ResourceKind::Pod)
                                .map(|pod| PodChild {
                                    kind: "pod".to_string(),
                                    name: pod.id.name.clone(),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    ServiceChild {
                        kind: "service".to_string(),
                        name: svc.id.name.clone(),
                        children: pods,
                    }
                })
                .collect();

            result.push(NamespaceHierarchy {
                namespace: ns_name.clone(),
                children: services,
            });
        }
        result
    }
}

impl Graph {
    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.key(), node);
    }

    pub fn add_edge(&mut self, parent: &NodeId, child: &NodeId) {
        let pkey = parent.key();
        let ckey = child.key();
        if self.nodes.contains_key(&pkey) && self.nodes.contains_key(&ckey) {
            let kind = self.nodes[&ckey].kind.clone();
            info!("Adding edge: {} -> {} ({:?})", pkey, ckey, kind);
            self.edges.entry(pkey.clone()).or_default().push(Edginfo {
                id: ckey.clone(),
                kind,
            });
            self.reverse.insert(ckey, pkey);
        }
    }

    pub fn remove_node(&mut self, id: &NodeId) {
        info!("Removing node: {}", id.key());
        let key = id.key();
        self.nodes.remove(&key);

        if let Some(children) = self.edges.remove(&key) {
            for edge in children {
                self.reverse.remove(&edge.id);
                if let Some(ns_id) = self.find_namespace_for(&edge.id) {
                    self.add_edge(
                        &ns_id,
                        &NodeId {
                            namespace: self.nodes[&edge.id].id.namespace.clone(),
                            name: self.nodes[&edge.id].id.name.clone(),
                        },
                    );
                }
            }
        }

        if let Some(parent) = self.reverse.remove(&key) {
            if let Some(children) = self.edges.get_mut(&parent) {
                children.retain(|edge| edge.id != key);
            }
        }
    }

    fn find_namespace_for(&self, child_key: &str) -> Option<NodeId> {
        let cnode = self.nodes.get(child_key)?;
        let ns = cnode.id.namespace.as_ref()?;
        self.nodes
            .values()
            .find(|n| n.kind == ResourceKind::Namespace && n.id.name == *ns)
            .map(|n| n.id.clone())
    }
}

struct Context {
    pub state: State,
    pub client: Client,
}

pub async fn run(state: State) {
    let client = Client::try_default()
        .await
        .expect("Failed to create K8s client");

    let context = state.to_context(client);

    let namespace_controller = Controller::new(
        Api::<Namespace>::all(context.client.clone()),
        Config::default().any_semantic(),
    )
    .shutdown_on_signal()
    .run(
        namespace_reconciler,
        namespace_error_policy,
        context.clone(),
    )
    .for_each(|res| async move {
        match res {
            Ok(_) => (),
            Err(e) => info!("namespace controller error: {:?}", e),
        }
    })
    .boxed();

    tokio::spawn(namespace_controller);

    let service_controller = Controller::new(
        Api::<Service>::all(context.client.clone()),
        Config::default().any_semantic(),
    )
    .shutdown_on_signal()
    .run(service_reconciler, service_error_policy, context.clone())
    .for_each(|res| async move {
        match res {
            Ok(_) => (),
            Err(e) => info!("service controller error: {:?}", e),
        }
    })
    .boxed();
    tokio::spawn(service_controller);

    let pod_controller = Controller::new(
        Api::<Pod>::all(context.client.clone()),
        Config::default().any_semantic(),
    )
    .shutdown_on_signal()
    .run(pod_reconciler, pod_error_policy, context.clone())
    .for_each(|res| async move {
        match res {
            Ok(_) => (),
            Err(e) => info!("controller error: {:?}", e),
        }
    })
    .boxed();

    tokio::spawn(pod_controller);
}

async fn pod_reconciler(pod: Arc<Pod>, ctx: Arc<Context>) -> Result<Action, kube::Error> {
    let name = match pod.metadata.name.as_deref() {
        Some(n) => n,
        None => return Ok(Action::await_change()),
    };
    let namespace = match pod.metadata.namespace.as_deref() {
        Some(ns) => ns,
        None => return Ok(Action::await_change()),
    };
    let spec = match pod.spec.clone() {
        Some(s) => Spec::PodSpec(s),
        None => return Ok(Action::await_change()),
    };

    let node_id = NodeId {
        namespace: Some(namespace.to_string()),
        name: name.to_string(),
    };
    let mut graph = ctx.state.graph.write().await;

    if pod.metadata.deletion_timestamp.is_some() {
        graph.remove_node(&node_id);
        return Ok(Action::await_change());
    }

    let pod_node = Node::new(
        ResourceKind::Pod,
        name,
        namespace,
        pod.metadata.clone(),
        spec.clone(),
    );
    graph.add_node(pod_node.clone());

    let empty_labels = BTreeMap::new();
    let pod_labels = pod.metadata.labels.as_ref().unwrap_or(&empty_labels);

    let service_to_attach = graph
        .nodes
        .values()
        .find(|svc_node| {
            if svc_node.kind != ResourceKind::Service {
                return false;
            }
            if svc_node.id.namespace.as_deref() != Some(namespace) {
                return false;
            }
            let selector = match &svc_node.spec {
                Spec::ServiceSpec(s) => s.selector.as_ref(),
                _ => return false,
            };
            if selector.is_none() {
                return false;
            }
            let sel = selector.unwrap();
            if sel.is_empty() {
                return false;
            }
            if !sel.iter().all(|(k, v)| pod_labels.get(k) == Some(v)) {
                return false;
            }
            if pod_labels
                .iter()
                .filter(|(k, _)| sel.contains_key(*k))
                .count()
                != sel.len()
            {
                return false;
            }
            true
        })
        .map(|s| s.id.clone());

    if let Some(parent_key) = graph.reverse.get(&node_id.key()).cloned() {
        if let Some(children) = graph.edges.get_mut(&parent_key) {
            children.retain(|e| e.id != node_id.key());
        }
        graph.reverse.remove(&node_id.key());
    }

    if let Some(svc_id) = service_to_attach {
        graph.add_edge(&svc_id, &node_id);
        return Ok(Action::await_change());
    }
    let ns_id = NodeId {
        namespace: Some(namespace.to_string()),
        name: namespace.to_string(),
    };
    if graph.nodes.contains_key(&ns_id.key()) {
        graph.add_edge(&ns_id, &node_id);
    }
    Ok(Action::await_change())
}

fn pod_error_policy(_pod: Arc<Pod>, error: &kube::Error, _ctx: Arc<Context>) -> Action {
    let error_str = format!("{:?}", error);
    if error_str.contains("peer closed connection without sending TLS close_notify") {
        return Action::requeue(Duration::from_secs(30));
    }
    error!(error = ?error, "pod reconcile failed");
    Action::requeue(Duration::from_secs(30))
}

async fn namespace_reconciler(
    namespace: Arc<Namespace>,
    ctx: Arc<Context>,
) -> Result<Action, kube::Error> {
    let name = match namespace.metadata.name.as_deref() {
        Some(n) => n,
        None => return Ok(Action::await_change()),
    };

    let node_id = NodeId {
        namespace: Some(name.to_string()),
        name: name.to_string(),
    };

    let node = Node::new(
        ResourceKind::Namespace,
        name,
        name,
        namespace.metadata.clone(),
        Spec::NamespaceSpec(namespace.spec.clone().unwrap_or_default()),
    );

    let mut graph = ctx.state.graph.write().await;
    graph.add_node(node);

    // Attach all existing non-Namespace nodes in this namespace
    let children: Vec<NodeId> = graph
        .nodes
        .values()
        .filter(|child| {
            child.id.namespace.as_deref() == Some(name)
                && child.kind != ResourceKind::Namespace
                && child.id != node_id // prevent self-loop
        })
        .map(|c| c.id.clone())
        .collect();

    for child_id in children {
        graph.add_edge(&node_id, &child_id);
    }

    info!("Reconciled Namespace {}", name);
    Ok(Action::await_change())
}

fn namespace_error_policy(
    _service: Arc<Namespace>,
    error: &kube::Error,
    _ctx: Arc<Context>,
) -> Action {
    let error_str = format!("{:?}", error);
    if error_str.contains("peer closed connection without sending TLS close_notify") {
        return Action::requeue(Duration::from_secs(30));
    }
    error!(error = ?error, "namespace reconcile failed");
    Action::requeue(Duration::from_secs(30))
}

async fn service_reconciler(
    service: Arc<Service>,
    ctx: Arc<Context>,
) -> Result<Action, kube::Error> {
    let name = match service.metadata.name.as_deref() {
        Some(n) => n,
        None => return Ok(Action::await_change()),
    };
    let namespace = match service.metadata.namespace.as_deref() {
        Some(ns) => ns,
        None => return Ok(Action::await_change()),
    };
    let spec = match service.spec.clone() {
        Some(s) => Spec::ServiceSpec(s),
        None => return Ok(Action::await_change()),
    };

    let node_id = NodeId {
        namespace: Some(namespace.to_string()),
        name: name.to_string(),
    };
    let mut graph = ctx.state.graph.write().await;

    if service.metadata.deletion_timestamp.is_some() {
        graph.remove_node(&node_id);
        return Ok(Action::await_change());
    }

    let ns_id = NodeId {
        namespace: Some(namespace.to_string()),
        name: namespace.to_string(),
    };
    graph.nodes.entry(ns_id.key()).or_insert_with(|| {
        Node::new(
            ResourceKind::Namespace,
            namespace,
            namespace,
            ObjectMeta {
                name: Some(namespace.to_string()),
                namespace: Some(namespace.to_string()),
                ..Default::default()
            },
            Spec::NamespaceSpec(v1::NamespaceSpec::default()),
        )
    });

    let svc_node = Node::new(
        ResourceKind::Service,
        name,
        namespace,
        service.metadata.clone(),
        spec.clone(),
    );
    graph.add_node(svc_node.clone());
    if let Some(parent_key) = graph.reverse.get(&node_id.key()).cloned() {
        if let Some(children) = graph.edges.get_mut(&parent_key) {
            children.retain(|e| e.id != node_id.key());
        }
        graph.reverse.remove(&node_id.key());
    }
    graph.add_edge(&ns_id, &node_id);

    let selector = match spec {
        Spec::ServiceSpec(ref s) => match &s.selector {
            Some(sel) => sel,
            None => return Ok(Action::await_change()),
        },
        _ => return Ok(Action::await_change()),
    };

    let empty_labels = BTreeMap::new();
    let pods_to_attach: Vec<NodeId> = graph
        .nodes
        .values()
        .filter(|pod| {
            pod.kind == ResourceKind::Pod
                && pod.id.namespace.as_deref() == Some(namespace)
                && selector.iter().all(|(k, v)| {
                    pod.metadata.labels.as_ref().unwrap_or(&empty_labels).get(k) == Some(v)
                })
        })
        .map(|p| p.id.clone())
        .collect();

    for pod_id in pods_to_attach {
        graph.add_edge(&node_id, &pod_id);

        if let Some(ns_for_pod) = graph.find_namespace_for(&pod_id.key()) {
            if let Some(children) = graph.edges.get_mut(&ns_for_pod.key()) {
                children.retain(|e| e.id != pod_id.key());
            }
            graph.reverse.remove(&pod_id.key());
        }
    }

    Ok(Action::await_change())
}

fn service_error_policy(_service: Arc<Service>, error: &kube::Error, _ctx: Arc<Context>) -> Action {
    let error_str = format!("{:?}", error);
    if error_str.contains("peer closed connection without sending TLS close_notify") {
        return Action::requeue(Duration::from_secs(30));
    }
    error!(error = ?error, "service reconcile failed");
    Action::requeue(Duration::from_secs(30))
}
