use crate::functional::{TestCluster, TestResources};
use serde_json::Value;
use tokio::time::{sleep, Duration};

use constellation::server::ConstellationServer;

#[cfg(feature = "functional-tests")]
#[tokio::test]
async fn test_simple_pod_creation_shows_in_state() -> Result<(), Box<dyn std::error::Error>> {
    let cluster = TestCluster::new().await?;
    cluster.wait_for_ready().await?;
    
    let resources = TestResources::new(cluster.client.clone(), "test-ns");
    resources.create_namespace().await?;
    
    let server = ConstellationServer::new_with_client("127.0.0.1:0", cluster.client.clone()).await?;
    let server_url = server.base_url();
    
    sleep(Duration::from_secs(3)).await;
    
    resources.create_test_deployment("test-app", 1).await?;
    resources.wait_for_pods_ready("app=test-app", 1).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    let response = reqwest::get(&format!("{}/state", server_url)).await?;
    assert!(response.status().is_success());
    
    let state_json: Value = response.json().await?;
    
    let found_pod = find_pod_in_state(&state_json, "test-app")?;
    assert!(found_pod, "Pod should appear in constellation state");
    
    let pod_phase = get_pod_phase(&state_json, "test-app")?;
    assert_eq!(pod_phase, "Running", "Pod should be in Running phase");
    
    resources.cleanup().await?;
    server.shutdown();
    
    Ok(())
}

fn find_pod_in_state(state: &Value, app_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let hierarchy = state["hierarchy"].as_array()
        .ok_or("No hierarchy found in state")?;
    
    for namespace in hierarchy {
        if find_pod_in_namespace(namespace, app_name) {
            return Ok(true);
        }
    }
    
    Ok(false)
}

fn find_pod_in_namespace(namespace: &Value, app_name: &str) -> bool {
    let Some(relatives) = namespace["relatives"].as_array() else {
        return false;
    };
    
    for resource in relatives {
        if is_matching_pod(resource, app_name) {
            return true;
        }
        
        if has_matching_pod_child(resource, app_name) {
            return true;
        }
    }
    
    false
}

fn is_matching_pod(resource: &Value, app_name: &str) -> bool {
    resource["kind"] == "Pod" && 
    resource["name"].as_str().map_or(false, |name| name.contains(app_name))
}

fn has_matching_pod_child(resource: &Value, app_name: &str) -> bool {
    let Some(pod_relatives) = resource["relatives"].as_array() else {
        return false;
    };
    
    pod_relatives.iter().any(|pod| is_matching_pod(pod, app_name))
}

fn get_pod_phase(state: &Value, app_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let hierarchy = state["hierarchy"].as_array()
        .ok_or("No hierarchy found in state")?;
    
    for namespace in hierarchy {
        if let Some(phase) = get_pod_phase_from_namespace(namespace, app_name) {
            return Ok(phase);
        }
    }
    
    Err("Pod not found in state".into())
}

fn get_pod_phase_from_namespace(namespace: &Value, app_name: &str) -> Option<String> {
    let relatives = namespace["relatives"].as_array()?;
    
    for resource in relatives {
        if let Some(phase) = get_pod_phase_from_resource(resource, app_name) {
            return Some(phase);
        }
    }
    
    None
}

fn get_pod_phase_from_resource(resource: &Value, app_name: &str) -> Option<String> {
    if is_matching_pod(resource, app_name) {
        return Some(resource["phase"].as_str().unwrap_or("Unknown").to_string());
    }
    
    let pod_relatives = resource["relatives"].as_array()?;
    for pod in pod_relatives {
        if is_matching_pod(pod, app_name) {
            return Some(pod["phase"].as_str().unwrap_or("Unknown").to_string());
        }
    }
    
    None
}