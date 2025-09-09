# Kubernetes Health Dashboard: Configuration Examples

This document provides example annotations and CRDs that could be used to configure a Kubernetes health dashboard and service topology visualization.

---

## 1. Annotations (Lightweight, Easy Adoption)

### Override Health Thresholds Per Service

```yaml
metadata:
  annotations:
    uptime.io/health-threshold: "80%"  # % of pods ready to consider UP
    uptime.io/degraded-latency-ms: "500"
```

### Mark Optional vs Required Dependencies

```yaml
metadata:
  annotations:
    uptime.io/optional-dep: "cache-service"
```

### UI Hints / Grouping

```yaml
metadata:
  annotations:
    uptime.io/group: "payments"
    uptime.io-display-name: "Payment API"
```

### Disable Monitoring for Certain Services

```yaml
metadata:
  annotations:
    uptime.io.ignore: "true"
```

---

## 2. CRDs (Structured, Declarative Configuration)

### ServiceHealthPolicy

```yaml
apiVersion: uptime.io/v1
kind: ServiceHealthPolicy
metadata:
  name: payments-api
spec:
  service: service-a
  minPodsReady: 2
  latencyThresholdMs: 500
  optionalDependencies:
    - cache-service
```

### ServiceDependency

```yaml
apiVersion: uptime.io/v1
kind: ServiceDependency
metadata:
  name: service-a-deps
spec:
  service: service-a
  dependsOn:
    - service-b
    - postgres
  optional: ["cache-service"]
```

### DashboardConfig

```yaml
apiVersion: uptime.io/v1
kind: DashboardConfig
spec:
  defaultDegradedThreshold: 0.8
  defaultImpactPropagation: true
  serviceGroups:
    - name: Payments
      services: ["service-a", "service-b"]
```

---

## 3. Implementation Proposal: Phase 1 - Annotation-Based Grouping

### Constellation Annotations

We'll use `constellation.io/` as the annotation prefix to match the project name:

```yaml
# Service grouping example
apiVersion: v1
kind: Service
metadata:
  name: payment-api
  annotations:
    constellation.io/group: "payments"
    constellation.io/display-name: "Payment API"
spec:
  # ... service spec
---
# Cache service (optional dependency)
apiVersion: v1
kind: Service
metadata:
  name: redis-cache
  annotations:
    constellation.io/group: "infrastructure" 
    constellation.io/display-name: "Redis Cache"
spec:
  # ... service spec
---
# Internal service to ignore
apiVersion: v1
kind: Service
metadata:
  name: internal-metrics
  annotations:
    constellation.io/ignore: "true"
spec:
  # ... service spec
```

### Backend Implementation Changes

**1. Update ResourceMetadata struct:**
```rust
#[derive(Debug, Clone, Serialize)]
pub struct ResourceMetadata {
    // ... existing fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub ignore: bool,
}
```

**2. Extract annotations in service processing:**
```rust
fn extract_service_annotations(metadata: &ObjectMeta) -> (Option<String>, Option<String>, bool) {
    let annotations = metadata.annotations.as_ref();
    
    let group = annotations
        .and_then(|a| a.get("constellation.io/group"))
        .map(|s| s.clone());
    
    let display_name = annotations
        .and_then(|a| a.get("constellation.io/display-name"))
        .map(|s| s.clone());
    
    let ignore = annotations
        .and_then(|a| a.get("constellation.io/ignore"))
        .map(|s| s == "true")
        .unwrap_or(false);
    
    (group, display_name, ignore)
}
```

**3. Filter ignored services in hierarchy building:**
```rust
// Skip services marked as ignored
if let Some(ResourceSpec::ServiceSpec(_)) = &service_node.spec {
    if service_node.resource_metadata.ignore {
        continue;
    }
}
```

### Frontend Implementation Changes

**1. Update ResourceNode type:**
```typescript
export interface ResourceNode {
    // ... existing fields
    group?: string;
    display_name?: string;
    ignore?: boolean;
}
```

**2. Group services by annotation in ResourceTree:**
```typescript
// Group services by their group annotation
const groupedServices = (services: ResourceNode[]) => {
    const groups = new Map<string, ResourceNode[]>();
    const ungrouped: ResourceNode[] = [];
    
    services.forEach(service => {
        if (service.group) {
            if (!groups.has(service.group)) {
                groups.set(service.group, []);
            }
            groups.get(service.group)!.push(service);
        } else {
            ungrouped.push(service);
        }
    });
    
    return { groups, ungrouped };
};
```

**3. Render groups with collapsible headers:**
```typescript
// In namespace rendering
{groups.size > 0 && Array.from(groups.entries()).map(([groupName, services]) => (
    <ServiceGroup
        key={groupName}
        name={groupName}
        services={services}
        level={level + 1}
        serviceSelectors={serviceSelectors}
        backendRefs={backendRefs}
    />
))}
{ungrouped.length > 0 && ungrouped.map(service => (
    <ServiceBox
        key={service.name}
        name={service.display_name || service.name}
        // ... other props
    />
))}
```

### UI Mockup

```
├── 📁 my-namespace (5 resources)
    ├── 🔽 payments (2 services)
    │   ├── 🟡 Payment API (payment-api)
    │   └── 🟡 Payment Worker (payment-worker)
    ├── 🔽 infrastructure (1 service)
    │   └── 🟡 Redis Cache (redis-cache)
    └── 🟡 user-service (ungrouped)
```

### Benefits of This Approach

1. **Zero breaking changes** - existing deployments continue to work
2. **Gradual adoption** - teams can add annotations as needed  
3. **Immediate value** - better organization without complex configuration
4. **Foundation for CRDs** - data model supports future CRD features
5. **GitOps friendly** - annotations live with the service definitions

### Future Extensions

Once annotation-based grouping is working, we can add:
- Cross-namespace groups via CRDs
- Group-level health policies  
- Dependency declarations between groups
- Custom group icons/colors

---

## 4. Recommended Approach

* **Start with annotations** for simplicity and minimal friction.
* **Internally generate structured objects** if advanced policies or dependency overrides are needed.
* **Introduce CRDs later** for full GitOps-driven configuration and multi-user policy management.
