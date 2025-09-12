use crate::functional::{TestCluster, TestResources};
use serde_json::Value;
use tokio::time::{Duration, sleep};

use constellation::server::Server;

#[cfg(feature = "functional-tests")]
#[tokio::test]
async fn test_comprehensive_resource_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = TestCluster::new().await?;
    let resources = TestResources::new(cluster.client.clone(), "test-ns");

    resources.create_namespace().await?;

    let server = Server::new_with_client("127.0.0.1:0", cluster.client.clone()).await?;
    let server_url = format!("http://{}", server.addr);
    let _server_handle = tokio::spawn(async move { server.serve().await });

    sleep(Duration::from_secs(6)).await;

    let mut service_annotations = std::collections::BTreeMap::new();
    service_annotations.insert("constellation.kyledev.co/group".to_string(), "backend".to_string());
    service_annotations.insert("constellation.kyledev.co/display-name".to_string(), "Test Backend".to_string());
    
    resources
        .create_test_service_with_annotations("test-service", "test-app", Some(service_annotations))
        .await?;
    resources.create_test_deployment("test-app", 1).await?;
    resources.wait_for_pods_ready("app=test-app", 1).await?;

    sleep(Duration::from_secs(3)).await;

    let response = reqwest::get(&format!("{}/state", server_url)).await?;
    let state: Value = response.json().await?;

    let hierarchy = state.as_array().unwrap();

    let namespace = hierarchy
        .iter()
        .find(|ns| ns["name"].as_str() == Some("test-ns"))
        .unwrap();

    let relatives = namespace["relatives"].as_array().unwrap();
    assert_eq!(relatives.len(), 1);

    let service = &relatives[0];
    assert_eq!(service["kind"].as_str().unwrap(), "Service");
    assert_eq!(service["name"].as_str().unwrap(), "test-service");
    
    assert_eq!(service["group"].as_str(), Some("backend"));
    assert_eq!(service["display_name"].as_str(), Some("Test Backend"));

    let pod_relatives = service["relatives"].as_array().unwrap();
    assert_eq!(pod_relatives.len(), 1);

    let pod = &pod_relatives[0];
    assert_eq!(pod["kind"].as_str().unwrap(), "Pod");
    assert!(pod["name"].as_str().unwrap().contains("test-app"));
    assert_eq!(pod["phase"].as_str().unwrap(), "Running");

    let pods = resources.get_pods("app=test-app").await?;
    let original_pod_name = pods[0].metadata.name.as_ref().unwrap();

    resources.delete_pod(original_pod_name).await?;
    resources.wait_for_pods_ready("app=test-app", 1).await?;

    sleep(Duration::from_secs(3)).await;

    let response2 = reqwest::get(&format!("{}/state", server_url)).await?;
    let final_state: Value = response2.json().await?;

    let final_hierarchy = final_state.as_array().unwrap();
    let final_namespace = final_hierarchy
        .iter()
        .find(|ns| ns["name"].as_str() == Some("test-ns"))
        .unwrap();
    let final_relatives = final_namespace["relatives"].as_array().unwrap();
    assert_eq!(final_relatives.len(), 1);

    let final_service = &final_relatives[0];
    assert_eq!(final_service["kind"].as_str().unwrap(), "Service");
    assert_eq!(final_service["name"].as_str().unwrap(), "test-service");
    
    assert_eq!(final_service["group"].as_str(), Some("backend"));
    assert_eq!(final_service["display_name"].as_str(), Some("Test Backend"));

    let final_pod_relatives = final_service["relatives"].as_array().unwrap();
    assert_eq!(final_pod_relatives.len(), 1);

    let final_pod = &final_pod_relatives[0];
    assert_eq!(final_pod["kind"].as_str().unwrap(), "Pod");
    assert!(final_pod["name"].as_str().unwrap().contains("test-app"));
    assert_eq!(final_pod["phase"].as_str().unwrap(), "Running");

    let final_pod_name = final_pod["name"].as_str().unwrap();
    assert_ne!(
        original_pod_name, final_pod_name,
        "New pod should have different name than deleted pod"
    );

    resources.cleanup().await?;
    cluster.cleanup().await?;
    Ok(())
}
