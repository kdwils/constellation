// use chrono::{DateTime, Utc};
// use futures::{Stream, StreamExt, TryStreamExt};
// use k8s_openapi::api::core::v1;
// use kube::{
//     api::Api, runtime::{
//         reflector::{self, Lookup, Store},
//         watcher::{self, Error as WatcherError}, 
//         WatchStreamExt,
//     }, Client, ResourceExt
// };

// use std::{collections::HashMap, sync::Arc};
// use tokio::sync::RwLock;

// #[derive(Clone)]
// pub struct State {
//     pub last_update: Arc<RwLock<DateTime<Utc>>>,
//     pub ingress_service_relationships: Arc<RwLock<HashMap<String, v1::Service>>>,
//     pub service_pod_relationships: Arc<RwLock<HashMap<String, Vec<v1::Pod>>>>,
// }

// impl Default for State {
//     fn default() -> Self {
//         Self {
//             last_update: Arc::new(RwLock::new(Utc::now())),
//             ingress_service_relationships: Arc::new(RwLock::new(HashMap::new())),
//             service_pod_relationships: Arc::new(RwLock::new(HashMap::new())),
//         }
//     }
// }

// impl State {}

// #[derive(Clone)]
// pub struct Context {
//     state: State,
//     pod_store: Store<v1::Pod>,
//     service_store: Store<v1::Service>,
// }

// pub async fn run(state: State) {
//     let client = Client::try_default()
//         .await
//         .expect("Failed to create K8s client");

//     let config = watcher::Config::default();

//     let pod_api: Api<v1::Pod> = Api::all(client.clone());
//     let (pod_store, pod_writer) = reflector::store::<v1::Pod>();
//     let pod_rf = reflector::reflector(
//         pod_writer, 
//         watcher::watcher(pod_api, config.clone()).default_backoff(),
//     );

//     let service_api: Api<v1::Service> = Api::all(client.clone());
//     let (service_store, service_writer) = reflector::store::<v1::Service>();
//     let service_rf = reflector::reflector(
//         service_writer,
//         watcher::watcher(service_api, config.clone()).default_backoff(),
//     );

//     let ctx: Context = Context {
//         state: state.clone(),
//         pod_store: pod_store.clone(),
//         service_store: service_store.clone(),
//     };

//     let pod_stream = Box::pin(pod_rf);
//     let service_stream = Box::pin(service_rf);

//     println!("Starting pod watcher");
//     tokio::spawn(pod_watcher(ctx.clone(), pod_stream));

//     println!("Starting service watcher");
//     tokio::spawn(service_watcher(ctx.clone(), service_stream));

//     ctx.pod_store.wait_until_ready().await.unwrap();
//     ctx.service_store.wait_until_ready().await.unwrap();

//     build_initial_relationships(ctx.clone()).await;
// }

// async fn build_initial_relationships(ctx: Context) {
//     println!("Building initial relationships between services and pods...");
//     let services_snapshot = ctx.service_store.state();
//     let pods_snapshot = ctx.pod_store.state();
    
//     println!("Found {} services and {} pods to process", services_snapshot.len(), pods_snapshot.len());
    
//     let mut relationships = ctx.state.service_pod_relationships.write().await;
    
//     for service in services_snapshot.iter() {
//         let (service_name, service_namespace, service_spec) = match (
//             service.metadata.name.as_deref(),
//             service.metadata.namespace.as_deref(),
//             service.spec.as_ref(),
//         ) {
//             (Some(name), Some(ns), Some(spec)) => (name, ns, spec),
//             _ => continue,
//         };

//         let selector = match service_spec.selector.as_ref() {
//             Some(s) => s,
//             None => continue,
//         };

//         let relationship_key = format!("{}/{}", service_namespace, service_name);
        
//         for pod in pods_snapshot.iter() {
//             let pod_namespace = match pod.metadata.namespace.as_deref() {
//                 Some(ns) => ns,
//                 None => continue,
//             };

//             if pod_namespace != service_namespace {
//                 continue;
//             }

//             let pod_labels = pod.metadata.labels.clone().unwrap_or_default();

//             // Check if pod matches service selector
//             let matches = selector.iter().all(|(key, value)| {
//                 pod_labels.get(key).map(|v| v == value).unwrap_or(false)
//             });

//             if matches {
//                 relationships.entry(relationship_key.clone()).or_insert_with(Vec::new).push(pod.as_ref().clone());
//                 println!("Initial relationship: {} -> {}/{}", 
//                     relationship_key, 
//                     pod_namespace,
//                     pod.metadata.name.as_deref().unwrap_or("unknown"));
//             }
//         }
//     }
    
//     println!("Initial relationship building complete. {} service keys created", relationships.len());
// }

// pub async fn pod_watcher<S>(ctx: Context, pod_stream: S)
// where
//     S: Stream<Item = Result<watcher::Event<v1::Pod>, WatcherError>> + Unpin,
// {

//     let watcher = pod_stream.touched_objects().for_each(|pod_event| async move {
//         match pod_event {
//             Ok(pod) => {
//                 let (pod_name, pod_namespace, pod_labels) = match (
//                     pod.metadata.name.as_ref(),
//                     pod.metadata.namespace.as_ref(),
//                     pod.metadata.labels.as_ref(),
//                 ) {
//                     (Some(name), Some(ns), Some(labels)) => (name.clone(), ns.clone(), labels.clone()),
//                     _ => {
//                         return;
//                     }
//                 };

//                 println!("Pod {}/{}", pod_namespace, pod_name);
//                 println!("Labels: {:?}", pod_labels);

                
//             }
//             Err(e) => {
//                 eprintln!("error from pod event: {:?}", e)
//             }
//         }
//     });
//     watcher.await;


//     // println!("Pod watcher started, waiting for events...");
//     // if let Err(e) = pod_stream
//     //     .applied_objects().for_each(|event| {
//     //         let ctx: Context = ctx.clone();
//     //         async move {
//     //             match event {
//     //                 watcher::Event::Apply(pod) => {
//     //                     let pod_name = match pod.metadata.name.as_deref() {
//     //                         Some(name) => name,
//     //                         None => {
//     //                             println!("Pod apply event: pod has no name, skipping");
//     //                             return Ok(());
//     //                         }
//     //                     };

//     //                     let pod_namespace = match pod.metadata.namespace.as_deref() {
//     //                         Some(ns) => ns,
//     //                         None => {
//     //                             println!("Pod apply event: pod {} has no namespace, skipping", pod_name);
//     //                             return Ok(());
//     //                         }
//     //                     };

//     //                     println!("Pod apply event: {}/{}", pod_namespace, pod_name);

//     //                     let pod_labels = pod.metadata.labels.clone().unwrap_or_default();
//     //                     let services_snapshot = ctx.service_store.state();
//     //                     println!("Checking pod against {} services", services_snapshot.len());

//     //                     let mut relationships = ctx.state.service_pod_relationships.write().await;
//     //                     let initial_count = relationships.len();

//     //                     // remove this pod from any existing service vectors
//     //                     relationships.retain(|_, pods_vec| {
//     //                         pods_vec.retain(|existing_pod| {
//     //                             existing_pod.metadata.name.as_deref() != Some(pod_name)
//     //                                 || existing_pod.metadata.namespace.as_deref() != Some(pod_namespace)
//     //                         });
//     //                         !pods_vec.is_empty()
//     //                     });
//     //                     let removed_count = initial_count - relationships.len();
//     //                     if removed_count > 0 {
//     //                         println!("Removed {} existing service keys (after cleaning pods) for pod", removed_count);
//     //                     }

//     //                     for service in services_snapshot.iter() {
//     //                         let (service_name, service_namespace, service_spec) = match (
//     //                             service.metadata.name.as_deref(),
//     //                             service.metadata.namespace.as_deref(),
//     //                             service.spec.as_ref(),
//     //                         ) {
//     //                             (Some(name), Some(ns), Some(spec)) => (name, ns, spec),
//     //                             _ => continue,
//     //                         };


//     //                         if service_namespace != pod_namespace {
//     //                             continue;
//     //                         }

//     //                         let selector = match service_spec.selector.as_ref() {
//     //                             Some(s) => s,
//     //                             None => continue,
//     //                         };

//     //                         let matches = selector.iter().all(|(key, value)| {
//     //                             pod_labels.get(key).map(|v| v == value).unwrap_or(false)
//     //                         });

//     //                         if matches {
//     //                             let relationship_key = format!("{}/{}", service_namespace, service_name);
//     //                             relationships.entry(relationship_key.clone()).or_insert_with(Vec::new).push(pod.clone());
//     //                             println!("Added relationship: {} -> {}/{}", relationship_key, pod_namespace, pod_name);
//     //                         }
//     //                     }
//     //                     let final_count = relationships.len();
//     //                     println!("Total service keys after processing: {}", final_count);
//     //                 }
//     //                 watcher::Event::Delete(pod) => {
//     //                     let pod_name = match pod.metadata.name.as_deref() {
//     //                         Some(name) => name,
//     //                         None => {
//     //                             println!("Pod delete event: pod has no name, skipping");
//     //                             return Ok(());
//     //                         }
//     //                     };

//     //                     let pod_namespace = match pod.metadata.namespace.as_deref() {
//     //                         Some(ns) => ns,
//     //                         None => {
//     //                             println!("Pod delete event: pod {} has no namespace, skipping", pod_name);
//     //                             return Ok(());
//     //                         }
//     //                     };

//     //                     println!("Pod delete event: {}/{}", pod_namespace, pod_name);

//     //                     let mut relationships = ctx.state.service_pod_relationships.write().await;
//     //                     let initial_count = relationships.len();

//     //                     // remove the deleted pod from all service vectors, drop empty keys
//     //                     relationships.retain(|_, pods_vec| {
//     //                         pods_vec.retain(|existing_pod| {
//     //                             !(existing_pod.metadata.name.as_deref() == Some(pod_name)
//     //                                 && existing_pod.metadata.namespace.as_deref() == Some(pod_namespace))
//     //                         });
//     //                         !pods_vec.is_empty()
//     //                     });

//     //                     let removed_count = initial_count - relationships.len();
//     //                     if removed_count > 0 {
//     //                         println!("Removed {} service keys after pod deletion", removed_count);
//     //                     }
//     //                     println!("Total service keys after deletion: {}", relationships.len());
//     //                 }
//     //                 _ => {}
//     //             }
//     //             Ok(())
//     //         }
//     //     })
//     //     .await
//     // {
//     //     eprintln!("pod watcher error: {}", e);
//     // }
// }

// pub async fn service_watcher<S>(ctx: Context, service_stream: S)
// where
//     S: Stream<Item = Result<watcher::Event<v1::Service>, WatcherError>> + Unpin,
// {
//     println!("Service watcher started, waiting for events...");
//     if let Err(e) = service_stream
//         .try_for_each(|event| {
//             let ctx = ctx.clone();
//             async move {
//                 match event {
//                     watcher::Event::Apply(service) => {
//                         let (service_name, service_namespace, service_spec) = match (
//                             service.metadata.name.as_deref(),
//                             service.metadata.namespace.as_deref(),
//                             service.spec.as_ref(),
//                         ) {
//                             (Some(name), Some(ns), Some(spec)) => (name, ns, spec),
//                             _ => {
//                                 println!("Service apply event: service missing name/namespace/spec, skipping");
//                                 return Ok(());
//                             }
//                         };

//                         println!("Service apply event: {}/{}", service_namespace, service_name);

//                         let selector = match service_spec.selector.as_ref() {
//                             Some(s) => s,
//                             None => {
//                                 println!("Service has no selector, skipping");
//                                 return Ok(());
//                             }
//                         };

//                         let pods_snapshot = ctx.pod_store.state();
//                         println!("Checking service against {} pods", pods_snapshot.len());
//                         let mut relationships = ctx.state.service_pod_relationships.write().await;
//                         let relationship_key = format!("{}/{}", service_namespace, service_name);

//                         // rebuild the vector for this service (remove old entry first)
//                         relationships.remove(&relationship_key);

//                         for pod in pods_snapshot.iter() {
//                             let pod_namespace = match pod.metadata.namespace.as_deref() {
//                                 Some(ns) => ns,
//                                 None => continue,
//                             };

//                             if pod_namespace != service_namespace {
//                                 continue;
//                             }

//                             let pod_labels = pod.metadata.labels.clone().unwrap_or_default();

//                             // Check if pod matches service selector
//                             let matches = selector.iter().all(|(key, value)| {
//                                 pod_labels.get(key).map(|v| v == value).unwrap_or(false)
//                             });

//                             if matches {
//                                 relationships.entry(relationship_key.clone()).or_insert_with(Vec::new).push(pod.as_ref().clone());
//                                 println!("Added relationship: {} -> {}/{}", 
//                                     relationship_key, 
//                                     pod.metadata.namespace.as_deref().unwrap_or("unknown"),
//                                     pod.metadata.name.as_deref().unwrap_or("unknown"));
//                             }
//                         }
//                         let final_count = relationships.len();
//                         println!("Total service keys after processing: {}", final_count);
//                     }
//                     watcher::Event::Delete(service) => {
//                         let (service_name, service_namespace) = match (
//                             service.metadata.name.as_deref(),
//                             service.metadata.namespace.as_deref(),
//                         ) {
//                             (Some(name), Some(ns)) => (name, ns),
//                             _ => {
//                                 println!("Service delete event: service missing name/namespace, skipping");
//                                 return Ok(());
//                             }
//                         };

//                         println!("Service delete event: {}/{}", service_namespace, service_name);

//                         let relationship_key = format!("{}/{}", service_namespace, service_name);
//                         let mut relationships = ctx.state.service_pod_relationships.write().await;
//                         let removed = relationships.remove(&relationship_key);
//                         if removed.is_some() {
//                             println!("Removed relationship for deleted service: {}", relationship_key);
//                         }
//                         println!("Total service keys after deletion: {}", relationships.len());
//                     }
//                     _ => {}
//                 }
//                 Ok(())
//             }
//         })
//         .await
//     {
//         eprintln!("service watcher error: {}", e);
//     }
// }
