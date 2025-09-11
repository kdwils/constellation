# Functional Test Suite Implementation Plan

## Overview
Create comprehensive functional tests using testcontainers-rs with k3s to validate pod phase synchronization fixes and prevent regression of the race condition issues described in the logs.

## Dependencies & Setup

### Add Dependencies
```toml
[dev-dependencies]
testcontainers-modules = { version = "0.14", features = ["k3s"] }
testcontainers = "0.23"
tokio-test = "0.4"
reqwest = { version = "0.12", features = ["json"] }
serde_yaml = "0.9"
```

### Test Directory Structure
```
tests/
├── functional/
│   ├── mod.rs              # Main test module
│   ├── cluster.rs          # K3s cluster management
│   ├── resources.rs        # K8s resource utilities
│   ├── pod_phases.rs       # Pod phase synchronization tests
│   └── race_conditions.rs  # Race condition simulation tests
└── fixtures/
    └── manifests/          # Test YAML manifests
```

## Core Components

### 1. K3s Cluster Management (`tests/functional/cluster.rs`)
- **TestCluster struct**: Manages k3s container lifecycle
- **Setup function**: Initialize k3s container with kubeconfig
- **Client creation**: Extract kubeconfig and create kube::Client
- **Cleanup**: Proper container teardown

### 2. Resource Utilities (`tests/functional/resources.rs`)
- **TestResources struct**: Namespace-scoped resource management
- **Deployment creation**: Create test deployments with configurable replicas
- **Service creation**: Create services with label selectors
- **Pod waiting**: Wait for pods to reach desired phases
- **Resource cleanup**: Delete test resources

### 3. Constellation Integration
- **Server startup**: Launch constellation server in test
- **Watcher startup**: Start watcher with test cluster client
- **State endpoint**: Query `/state/` for hierarchy validation
- **JSON parsing**: Extract pod phases from constellation state

## Test Scenarios

### 1. Basic Pod Phase Tests (`tests/functional/pod_phases.rs`)

#### `test_pod_creation_shows_correct_phase`
- Create deployment with 1 replica
- Wait for pod to become Running
- Verify constellation state shows Running phase
- Assert no stale Pending phases exist

#### `test_pod_recreation_updates_phase`
- Create deployment
- Wait for Running state
- Restart deployment (simulates pod recreation)
- Verify new pods show Running, not Pending
- Assert old pod entries are removed

#### `test_pod_deletion_removes_from_hierarchy`
- Create deployment
- Verify pod appears in constellation state
- Delete deployment
- Assert pod is removed from hierarchy

### 2. Race Condition Tests (`tests/functional/race_conditions.rs`)

#### `test_rapid_pod_apply_events`
- Create deployment with multiple replicas
- Simulate rapid pod recreation (matches log pattern):
  ```
  pod applied: blog-58d84b9659-pfskd (multiple rapid events)
  pod applied: blog-58d84b9659-795k9 (multiple rapid events)  
  pod deleted: blog-58d84b9659-pfskd
  pod applied: blog-58d84b9659-795k9 (final event)
  ```
- Verify constellation state shows correct final phases
- Assert no duplicate or stale pod entries

#### `test_concurrent_pod_operations`
- Create multiple deployments simultaneously
- Perform concurrent pod operations (delete/create)
- Verify constellation maintains consistent state
- Assert no race conditions cause incorrect phases

#### `test_out_of_order_events`
- Manually create pod events in specific sequences
- Verify update_pod_relationships handles all scenarios
- Test edge cases from the original bug report

### 3. Integration Tests (`tests/functional/mod.rs`)

#### `test_constellation_end_to_end`
- Full constellation server + watcher + k3s cluster
- Create complex resource hierarchy (namespace → httproute → service → pods)
- Perform various operations (create, update, delete)
- Validate complete system behavior
- Test HTTP API responses

#### `test_service_selector_matching`
- Create services with different selectors
- Create pods with various labels
- Verify pods appear under correct services
- Test label changes and selector updates

## Implementation Steps

### Phase 1: Infrastructure
1. Add testcontainers dependencies to Cargo.toml
2. Create test directory structure
3. Implement TestCluster with k3s setup
4. Create TestResources utility functions
5. Add constellation server/watcher integration

### Phase 2: Basic Tests
1. Implement simple pod creation/deletion tests
2. Add constellation state validation helpers
3. Create JSON parsing utilities for hierarchy
4. Test basic watcher functionality

### Phase 3: Race Condition Tests
1. Implement rapid event simulation
2. Create concurrent operation tests
3. Add specific test cases matching log patterns
4. Verify update_pod_relationships fix works

### Phase 4: Advanced Scenarios
1. Add complex resource hierarchy tests
2. Implement service selector matching tests
3. Create HTTPRoute backend reference tests
4. Add performance and memory leak tests

### Phase 5: CI Integration
1. Configure GitHub Actions for testcontainers
2. Set up Docker-in-Docker if needed
3. Add test parallelization
4. Configure test reporting and artifacts

## Success Criteria

- ✅ All tests pass consistently
- ✅ Race condition from logs is reproduced and fixed
- ✅ No stale pod phases appear in constellation state
- ✅ Pod recreation events are handled correctly
- ✅ Multiple rapid Apply events don't cause duplicates
- ✅ System maintains correct state under concurrent operations
- ✅ Tests run in CI/CD without external dependencies

## Benefits

- **Real Kubernetes Environment**: Tests against actual k8s API
- **Reproducible Issues**: Can simulate exact race conditions
- **Regression Prevention**: Catches future pod phase bugs
- **Fast Feedback**: No external cluster dependencies
- **Comprehensive Coverage**: Tests entire watcher → state → API flow

This plan provides a complete testing framework to validate the pod phase synchronization fix and prevent similar issues in the future.