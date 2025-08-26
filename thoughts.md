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

## 3. Recommended Approach

* **Start with annotations** for simplicity and minimal friction.
* **Internally generate structured objects** if advanced policies or dependency overrides are needed.
* **Introduce CRDs later** for full GitOps-driven configuration and multi-user policy management.
