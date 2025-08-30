// use futures::{FutureExt, StreamExt};
// use gateway_api::httproutes::{HTTPRoute, HTTPRouteSpec};
// use k8s_openapi::api::core::v1::{self, Namespace, Pod, Service};
// use kube::{
//     Client,
//     api::{Api, ObjectMeta},
//     runtime::{Controller, controller::Action, watcher::Config},
// };
// use serde;
// use serde::Serialize;
// use std::{collections::BTreeMap, sync::Arc, time::Duration};
// use tokio::sync::RwLock;
// use tracing::{error, info};

// #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
// pub enum ResourceKind {
//     Namespace,
//     Service,
//     Pod,
//     HTTPRoute,
// }

// #[derive(Debug, Clone)]
// pub enum ResourceSpec {
//     NamespaceSpec(v1::NamespaceSpec),
//     ServiceSpec(v1::ServiceSpec),
//     PodSpec(v1::PodSpec),
//     HTTPRouteSpec(HTTPRouteSpec),
// }

// #[derive(Debug, Clone, Serialize)]
// pub struct HierarchyNode {
//     pub kind: ResourceKind,
//     pub name: String,
//     pub children: Vec<HierarchyNode>,
//     #[serde(skip)]
//     pub metadata: ObjectMeta,
//     #[serde(skip)]
//     pub spec: Option<ResourceSpec>,
// }

// impl HierarchyNode {
//     pub fn new(
//         kind: ResourceKind,
//         name: impl Into<String>,
//         metadata: ObjectMeta,
//         spec: Option<ResourceSpec>,
//     ) -> Self {
//         Self {
//             kind,
//             name: name.into(),
//             children: Vec::new(),
//             metadata,
//             spec,
//         }
//     }
// }

// #[derive(Clone)]
// pub struct State {
//     pub hierarchy: Arc<RwLock<Vec<HierarchyNode>>>,
// }

// impl State {
//     pub fn default() -> Self {
//         Self {
//             hierarchy: Arc::new(RwLock::new(Vec::new())),
//         }
//     }
//     fn to_context(&self, client: Client) -> Arc<Context> {
//         Arc::new(Context {
//             state: self.clone(),
//             client,
//         })
//     }
// }

// struct Context {
//     pub state: State,
//     pub client: Client,
// }

// pub async fn run(state: State) {
//     let client = Client::try_default()
//         .await
//         .expect("Failed to create K8s client");
//     let context = state.to_context(client);

//     let namespace_controller = Controller::new(
//         Api::<Namespace>::all(context.client.clone()),
//         Config::default().any_semantic(),
//     )
//     .shutdown_on_signal()
//     .run(
//         namespace_reconciler,
//         namespace_error_policy,
//         context.clone(),
//     )
//     .for_each(|res| async move {
//         match res {
//             Ok(_) => (),
//             Err(e) => info!("namespace controller error: {:?}", e),
//         }
//     })
//     .boxed();
//     tokio::spawn(namespace_controller);

//     let httproute_controller = Controller::new(
//         Api::<HTTPRoute>::all(context.client.clone()),
//         Config::default().any_semantic(),
//     )
//     .shutdown_on_signal()
//     .run(
//         httproute_reconciler,
//         httproute_error_policy,
//         context.clone(),
//     )
//     .for_each(|res| async move {
//         match res {
//             Ok(_) => (),
//             Err(e) => info!("namespace controller error: {:?}", e),
//         }
//     })
//     .boxed();
//     tokio::spawn(httproute_controller);

//     let service_controller = Controller::new(
//         Api::<Service>::all(context.client.clone()),
//         Config::default().any_semantic(),
//     )
//     .shutdown_on_signal()
//     .run(service_reconciler, service_error_policy, context.clone())
//     .for_each(|res| async move {
//         match res {
//             Ok(_) => (),
//             Err(e) => info!("service controller error: {:?}", e),
//         }
//     })
//     .boxed();
//     tokio::spawn(service_controller);

//     let pod_controller = Controller::new(
//         Api::<Pod>::all(context.client.clone()),
//         Config::default().any_semantic(),
//     )
//     .shutdown_on_signal()
//     .run(pod_reconciler, pod_error_policy, context.clone())
//     .for_each(|res| async move {
//         match res {
//             Ok(_) => (),
//             Err(e) => info!("pod controller error: {:?}", e),
//         }
//     })
//     .boxed();
//     tokio::spawn(pod_controller);
// }

// async fn namespace_reconciler(
//     namespace: Arc<Namespace>,
//     ctx: Arc<Context>,
// ) -> Result<Action, kube::Error> {
//     let name = match namespace.metadata.name.as_deref() {
//         Some(n) => n,
//         None => return Ok(Action::await_change()),
//     };
//     let mut hierarchy = ctx.state.hierarchy.write().await;
//     if !hierarchy
//         .iter()
//         .any(|n| n.name == name && n.kind == ResourceKind::Namespace)
//     {
//         let spec = namespace.spec.clone().map(ResourceSpec::NamespaceSpec);
//         hierarchy.push(HierarchyNode::new(
//             ResourceKind::Namespace,
//             name,
//             namespace.metadata.clone(),
//             spec,
//         ));
//     }
//     info!("Reconciled Namespace {}", name);
//     Ok(Action::await_change())
// }

// fn httproute_error_policy(
//     _httproute: Arc<HTTPRoute>,
//     error: &kube::Error,
//     _ctx: Arc<Context>,
// ) -> Action {
//     error!(error = ?error, "httproute reconcile failed");
//     Action::requeue(Duration::from_secs(30))
// }

// fn namespace_error_policy(_ns: Arc<Namespace>, error: &kube::Error, _ctx: Arc<Context>) -> Action {
//     error!(error = ?error, "namespace reconcile failed");
//     Action::requeue(Duration::from_secs(30))
// }

// fn service_error_policy(_ns: Arc<Service>, error: &kube::Error, _ctx: Arc<Context>) -> Action {
//     error!(error = ?error, "namespace reconcile failed");
//     Action::requeue(Duration::from_secs(30))
// }

// async fn service_reconciler(
//     service: Arc<Service>,
//     ctx: Arc<Context>,
// ) -> Result<Action, kube::Error> {
//     let name = match service.metadata.name.as_deref() {
//         Some(n) => n,
//         None => return Ok(Action::await_change()),
//     };
//     let namespace = match service.metadata.namespace.as_deref() {
//         Some(ns) => ns,
//         None => return Ok(Action::await_change()),
//     };
//     let spec = service.spec.clone().map(ResourceSpec::ServiceSpec);

//     let mut hierarchy = ctx.state.hierarchy.write().await;

//     let route_indices: Vec<usize> = hierarchy
//         .iter()
//         .enumerate()
//         .filter(|(_, hr)| {
//             hr.kind == ResourceKind::HTTPRoute
//                 && hr.metadata.namespace.as_deref() == Some(namespace)
//                 && match &hr.spec {
//                     Some(ResourceSpec::HTTPRouteSpec(spec)) => spec
//                         .parent_refs
//                         .as_ref()
//                         .map_or(false, |refs| refs.iter().any(|pr| pr.name == name)),
//                     _ => false,
//                 }
//         })
//         .map(|(i, _)| i)
//         .collect();

//     let ns_index = match hierarchy
//         .iter()
//         .position(|n| n.name == namespace && n.kind == ResourceKind::Namespace)
//     {
//         Some(i) => i,
//         None => {
//             hierarchy.push(HierarchyNode::new(
//                 ResourceKind::Namespace,
//                 namespace,
//                 ObjectMeta {
//                     name: Some(namespace.to_string()),
//                     namespace: Some(namespace.to_string()),
//                     ..Default::default()
//                 },
//                 None,
//             ));
//             hierarchy.len() - 1
//         }
//     };

//     if !route_indices.is_empty() {
//         for &i in &route_indices {
//             let route_node = &mut hierarchy[i];
//             if !route_node
//                 .children
//                 .iter()
//                 .any(|c| c.name == name && c.kind == ResourceKind::Service)
//             {
//                 route_node.children.push(HierarchyNode::new(
//                     ResourceKind::Service,
//                     name,
//                     service.metadata.clone(),
//                     spec.clone(),
//                 ));
//             }
//         }
//         return Ok(Action::await_change());
//     }

//     let ns_node = &mut hierarchy[ns_index];

//     if let Some(svc_node) = ns_node
//         .children
//         .iter_mut()
//         .find(|c| c.name == name && c.kind == ResourceKind::Service)
//     {
//         svc_node.metadata = service.metadata.clone();
//         svc_node.spec = spec;
//         return Ok(Action::await_change());
//     }

//     ns_node.children.push(HierarchyNode::new(
//         ResourceKind::Service,
//         name,
//         service.metadata.clone(),
//         spec.clone(),
//     ));

//     Ok(Action::await_change())
// }

// async fn pod_reconciler(pod: Arc<Pod>, ctx: Arc<Context>) -> Result<Action, kube::Error> {
//     let name = match pod.metadata.name.as_deref() {
//         Some(n) => n,
//         None => return Ok(Action::await_change()),
//     };

//     let namespace = match pod.metadata.namespace.as_deref() {
//         Some(ns) => ns,
//         None => return Ok(Action::await_change()),
//     };

//     let spec = pod.spec.clone().map(ResourceSpec::PodSpec);
//     let pod_labels = match pod.metadata.labels.as_ref() {
//         Some(l) => l,
//         None => &BTreeMap::new(),
//     };

//     let mut hierarchy = ctx.state.hierarchy.write().await;

//     let ns_node = match hierarchy
//         .iter_mut()
//         .find(|n| n.name == namespace && n.kind == ResourceKind::Namespace)
//     {
//         Some(node) => node,
//         None => return Ok(Action::await_change()),
//     };

//     let mut attached = false;
//     for svc_node in ns_node
//         .children
//         .iter_mut()
//         .filter(|c| c.kind == ResourceKind::Service)
//     {
//         let s = match &svc_node.spec {
//             Some(ResourceSpec::ServiceSpec(s)) => s,
//             _ => continue,
//         };

//         let selector = match &s.selector {
//             Some(sel) if !sel.is_empty() => sel,
//             _ => continue,
//         };

//         if selector.iter().all(|(k, v)| pod_labels.get(k) == Some(v)) {
//             svc_node.children.push(HierarchyNode::new(
//                 ResourceKind::Pod,
//                 name,
//                 pod.metadata.clone(),
//                 spec.clone(),
//             ));
//             attached = true;
//         }
//     }

//     if !attached {
//         ns_node.children.push(HierarchyNode::new(
//             ResourceKind::Pod,
//             name,
//             pod.metadata.clone(),
//             spec.clone(),
//         ));
//     }

//     Ok(Action::await_change())
// }

// async fn httproute_reconciler(
//     route: Arc<HTTPRoute>,
//     ctx: Arc<Context>,
// ) -> Result<Action, kube::Error> {
//     let route_name = match route.metadata.name.as_deref() {
//         Some(n) => n,
//         None => return Ok(Action::await_change()),
//     };
//     let namespace = match route.metadata.namespace.as_deref() {
//         Some(ns) => ns,
//         None => return Ok(Action::await_change()),
//     };
//     let route_spec = Some(ResourceSpec::HTTPRouteSpec(route.spec.clone()));

//     let mut hierarchy = ctx.state.hierarchy.write().await;

//     let ns_index = match hierarchy
//         .iter()
//         .position(|n| n.name == namespace && n.kind == ResourceKind::Namespace)
//     {
//         Some(i) => i,
//         None => {
//             hierarchy.push(HierarchyNode::new(
//                 ResourceKind::Namespace,
//                 namespace,
//                 ObjectMeta {
//                     name: Some(namespace.to_string()),
//                     namespace: Some(namespace.to_string()),
//                     ..Default::default()
//                 },
//                 None,
//             ));
//             hierarchy.len() - 1
//         }
//     };

//     let ns_node = &mut hierarchy[ns_index];

//     let route_index = match ns_node
//         .children
//         .iter()
//         .position(|c| c.name == route_name && c.kind == ResourceKind::HTTPRoute)
//     {
//         Some(i) => i,
//         None => {
//             ns_node.children.push(HierarchyNode::new(
//                 ResourceKind::HTTPRoute,
//                 route_name,
//                 route.metadata.clone(),
//                 route_spec.clone(),
//             ));
//             ns_node.children.len() - 1
//         }
//     };

//     let services_to_attach: Vec<HierarchyNode> = ns_node
//         .children
//         .iter()
//         .filter(|child| {
//             child.kind == ResourceKind::Service
//                 && route_spec.as_ref().and_then(|r| {
//                     if let ResourceSpec::HTTPRouteSpec(spec) = r {
//                         Some(
//                             spec.parent_refs
//                                 .as_ref()
//                                 .map_or(false, |refs| refs.iter().any(|pr| pr.name == child.name)),
//                         )
//                     } else {
//                         None
//                     }
//                 }) == Some(true)
//         })
//         .cloned()
//         .collect();

//     info!(services = ?services_to_attach);

//     // Step 2: Mutable borrow to route node
//     let route_node = &mut ns_node.children[route_index];
//     route_node.metadata = route.metadata.clone();
//     route_node.spec = route_spec.clone();

//     // Step 3: Attach cloned service nodes
//     for svc_node in services_to_attach {
//         if !route_node.children.iter().any(|c| c.name == svc_node.name) {
//             route_node.children.push(svc_node);
//         }
//     }

//     Ok(Action::await_change())
// }

// // async fn httproute_reconciler(
// //     route: Arc<HTTPRoute>,
// //     ctx: Arc<Context>,
// // ) -> Result<Action, kube::Error> {
// //     let route_name = match route.metadata.name.as_deref() {
// //         Some(n) => n,
// //         None => return Ok(Action::await_change()),
// //     };

// //     let namespace = match route.metadata.namespace.as_deref() {
// //         Some(ns) => ns,
// //         None => return Ok(Action::await_change()),
// //     };

// //     let route_spec = Some(ResourceSpec::HTTPRouteSpec(route.spec.clone()));

// //     let mut hierarchy = ctx.state.hierarchy.write().await;

// //     let ns_node = match hierarchy
// //         .iter_mut()
// //         .find(|n| n.name == namespace && n.kind == ResourceKind::Namespace)
// //     {
// //         Some(node) => node,
// //         None => return Ok(Action::await_change()),
// //     };

// //     let route_node: &mut HierarchyNode = match ns_node
// //         .children
// //         .iter_mut()
// //         .find(|c| c.name == route_name && c.kind == ResourceKind::HTTPRoute)
// //     {
// //         Some(node) => {
// //             node.metadata = route.metadata.clone();
// //             node.spec = route_spec.clone();
// //             node
// //         }
// //         None => {
// //             ns_node.children.push(HierarchyNode::new(
// //                 ResourceKind::HTTPRoute,
// //                 route_name,
// //                 route.metadata.clone(),
// //                 route_spec.clone(),
// //             ));
// //             ns_node.children.last_mut().unwrap()
// //         }
// //     };

// //     let service_indices: Vec<usize> = ns_node
// //         .children
// //         .iter()
// //         .enumerate()
// //         .filter(|(_, svc)| {
// //             svc.kind == ResourceKind::Service
// //                 && match route_node.spec.clone() {
// //                     Some(ResourceSpec::HTTPRouteSpec(spec)) => spec
// //                         .parent_refs
// //                         .as_ref()
// //                         .map_or(false, |refs| refs.iter().any(|pr| pr.name == svc.name)),
// //                     _ => false,
// //                 }
// //         })
// //         .map(|(i, _)| i)
// //         .collect();

// //     for idx in service_indices {
// //         let svc_node = ns_node.children[idx].clone();
// //         if !route_node.children.iter().any(|c| c.name == svc_node.name) {
// //             route_node.children.push(svc_node);
// //         }
// //     }

// //     Ok(Action::await_change())
// // }

// fn pod_error_policy(_pod: Arc<Pod>, error: &kube::Error, _ctx: Arc<Context>) -> Action {
//     error!(error = ?error, "pod reconcile failed");
//     Action::requeue(Duration::from_secs(30))
// }
