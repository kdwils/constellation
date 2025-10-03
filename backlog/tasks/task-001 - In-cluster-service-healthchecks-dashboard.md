---
id: task-001
title: In cluster service healthchecks dashboard
status: In Progress
assignee:
  - '@claude'
created_date: '2025-09-26 21:12'
updated_date: '2025-09-26 22:08'
labels: []
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
This feature aims to add health check history and a dashboard view for in cluster services.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Track in-cluster service uptime by default
- [ ] #2 Landing page on the dashboard shows up time for all in-cluster services
- [ ] #3 Ignore annotations are respected
- [ ] #4 HTTP Get healthchecks are implemented
- [ ] #5 Health check history is stored in memory as part of the MVP
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
A new state manager go routine needs to perform health checks based on the readiness probes defined by a pod. It should respect the settings on the health probe, or use safe defaults for how often to perform the healthcheck.

The statement manager should use the following HTTP client definition for dependency injection during tests
type HTTPClient interface {
	Do(*http.Request) (*http.Response, error)
}

To start, we can begin with just accepting 200 OK as a healthy response.

To build out the endpoints to hit, we will be using the state within the manager on each interval.

If a service my-service spec exists in my namespace
spec:
  ports:
  - port: 80
    protocol: TCP
    targetPort: 8080

and the spec for a pod in my-namespace
containers:
    ports:
      - name: h2c
        containerPort: 8080
        protocol: TCP
    livenessProbe:
      grpc:
        port: 8080

The health point checker needs to hit my-service.my-namespace.svc.cluster.local:80 because port 80 of the service maps to 8080 on the pod, and 8080 on the pod is what the probe defines to hit.
<!-- SECTION:PLAN:END -->
