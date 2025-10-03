package controller

import (
	"context"
	"fmt"
	"net/http"
	"net/url"
	"sync"
	"time"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/util/intstr"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"

	"github.com/kdwils/constellation/internal/types"
)

const (
	defaultCheckInterval    = 30 * time.Second
	defaultRequestTimeout   = 10 * time.Second
	maxHistorySize          = 100
	healthIgnoreAnnotation  = "constellation.kyledev.co/ignore"
)

// HTTPClient interface for dependency injection during tests
type HTTPClient interface {
	Do(*http.Request) (*http.Response, error)
}

// ProbeInfo contains information about a pod's health probe
type ProbeInfo struct {
	ServiceName   string
	Namespace     string
	PodName       string
	ContainerName string
	Port          int32
	Path          string
	Scheme        string
	ServicePort   int32
	Interval      time.Duration
	Timeout       time.Duration
}

// HealthChecker manages health checks for in-cluster services based on pod probes
type HealthChecker struct {
	client       client.Client
	httpClient   HTTPClient
	stateManager StateManagerInterface
	mu           sync.RWMutex
	healthData   map[string]*types.ServiceHealthInfo
	probeInfo    map[string][]ProbeInfo // serviceKey -> probe infos
	stopChan     chan struct{}
	interval     time.Duration
}

// StateManagerInterface defines the interface needed by HealthChecker
type StateManagerInterface interface {
	GetHierarchy() []types.HierarchyNode
	UpdateServiceHealth(serviceName, namespace string, healthInfo *types.ServiceHealthInfo)
}

// NewHealthChecker creates a new health checker
func NewHealthChecker(client client.Client, stateManager StateManagerInterface) *HealthChecker {
	return &HealthChecker{
		client:       client,
		httpClient:   &http.Client{Timeout: defaultRequestTimeout},
		stateManager: stateManager,
		healthData:   make(map[string]*types.ServiceHealthInfo),
		probeInfo:    make(map[string][]ProbeInfo),
		stopChan:     make(chan struct{}),
		interval:     defaultCheckInterval,
	}
}

// Start begins the health checking routine
func (hc *HealthChecker) Start(ctx context.Context) error {
	logger := log.FromContext(ctx)
	logger.Info("Starting health checker")

	ticker := time.NewTicker(hc.interval)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			hc.performHealthChecks(ctx)
		case <-ctx.Done():
			logger.Info("Health checker stopping")
			return nil
		case <-hc.stopChan:
			logger.Info("Health checker stopped")
			return nil
		}
	}
}

// Stop stops the health checker
func (hc *HealthChecker) Stop() {
	close(hc.stopChan)
}

// performHealthChecks discovers probes and checks all services
func (hc *HealthChecker) performHealthChecks(ctx context.Context) {
	logger := log.FromContext(ctx)
	
	// First, discover all probe configurations
	if err := hc.discoverProbes(ctx); err != nil {
		logger.Error(err, "Failed to discover probes")
		return
	}

	// Then perform health checks
	hc.mu.RLock()
	probeInfoCopy := make(map[string][]ProbeInfo)
	for k, v := range hc.probeInfo {
		probeInfoCopy[k] = v
	}
	hc.mu.RUnlock()

	for serviceKey, probes := range probeInfoCopy {
		for _, probe := range probes {
			hc.checkServiceProbe(ctx, serviceKey, probe)
		}
	}

	logger.V(1).Info("Health check cycle completed")
}

// discoverProbes scans the hierarchy to find services and their associated pod probes
func (hc *HealthChecker) discoverProbes(ctx context.Context) error {
	hierarchy := hc.stateManager.GetHierarchy()
	
	hc.mu.Lock()
	// Clear existing probe info
	hc.probeInfo = make(map[string][]ProbeInfo)
	hc.mu.Unlock()

	for _, namespace := range hierarchy {
		if namespace.Kind != types.ResourceKindNamespace {
			continue
		}
		if err := hc.discoverProbesInNamespace(ctx, namespace); err != nil {
			return err
		}
	}

	return nil
}

// discoverProbesInNamespace discovers probes for services in a namespace
func (hc *HealthChecker) discoverProbesInNamespace(ctx context.Context, namespace types.HierarchyNode) error {
	return hc.walkHierarchyForProbes(ctx, namespace.Relatives, namespace.Name)
}

// walkHierarchyForProbes recursively walks hierarchy to find services and extract probe info
func (hc *HealthChecker) walkHierarchyForProbes(ctx context.Context, nodes []types.HierarchyNode, namespaceName string) error {
	for _, node := range nodes {
		if node.Kind == types.ResourceKindService && !node.Ignore {
			if err := hc.extractServiceProbes(ctx, node, namespaceName); err != nil {
				return err
			}
		}
		if len(node.Relatives) > 0 {
			if err := hc.walkHierarchyForProbes(ctx, node.Relatives, namespaceName); err != nil {
				return err
			}
		}
	}
	return nil
}

// extractServiceProbes extracts probe information from pods backing a service
func (hc *HealthChecker) extractServiceProbes(ctx context.Context, serviceNode types.HierarchyNode, namespaceName string) error {
	logger := log.FromContext(ctx)
	serviceKey := fmt.Sprintf("%s/%s", namespaceName, serviceNode.Name)

	// Get the actual service to understand port mappings
	var service corev1.Service
	err := hc.client.Get(ctx, client.ObjectKey{
		Name:      serviceNode.Name,
		Namespace: namespaceName,
	}, &service)
	if err != nil {
		logger.Error(err, "Failed to get service", "service", serviceNode.Name, "namespace", namespaceName)
		return err
	}

	if shouldIgnoreHealthCheck(service.Annotations) {
		return nil
	}

	// Find pods that match this service
	probes := []ProbeInfo{}
	for _, relative := range serviceNode.Relatives {
		if relative.Kind == types.ResourceKindPod {
			podProbes, err := hc.extractPodProbes(ctx, relative, namespaceName, service)
			if err != nil {
				logger.Error(err, "Failed to extract pod probes", "pod", relative.Name)
				continue
			}
			probes = append(probes, podProbes...)
		}
	}

	if len(probes) > 0 {
		hc.mu.Lock()
		hc.probeInfo[serviceKey] = probes
		hc.mu.Unlock()
		logger.V(1).Info("Discovered probes for service", "service", serviceNode.Name, "namespace", namespaceName, "probeCount", len(probes))
	}

	return nil
}

// extractPodProbes extracts probe information from a single pod
func (hc *HealthChecker) extractPodProbes(ctx context.Context, podNode types.HierarchyNode, namespaceName string, service corev1.Service) ([]ProbeInfo, error) {
	var pod corev1.Pod
	err := hc.client.Get(ctx, client.ObjectKey{
		Name:      podNode.Name,
		Namespace: namespaceName,
	}, &pod)
	if err != nil {
		return nil, err
	}

	var probes []ProbeInfo

	for _, container := range pod.Spec.Containers {
		// Check readiness probe first (preferred for health checks)
		if container.ReadinessProbe != nil && container.ReadinessProbe.HTTPGet != nil {
			probe := hc.buildProbeInfo(service, pod, container, container.ReadinessProbe)
			if probe != nil {
				probes = append(probes, *probe)
			}
		}
		// Fall back to liveness probe if no readiness probe
		if container.ReadinessProbe == nil && container.LivenessProbe != nil && container.LivenessProbe.HTTPGet != nil {
			probe := hc.buildProbeInfo(service, pod, container, container.LivenessProbe)
			if probe != nil {
				probes = append(probes, *probe)
			}
		}
	}

	return probes, nil
}

// buildProbeInfo constructs ProbeInfo from service, pod, and probe configuration
func (hc *HealthChecker) buildProbeInfo(service corev1.Service, pod corev1.Pod, container corev1.Container, probe *corev1.Probe) *ProbeInfo {
	httpGet := probe.HTTPGet
	if httpGet == nil {
		return nil
	}

	// Find the container port that matches the probe port
	containerPort := hc.resolveContainerPort(container, httpGet.Port)
	if containerPort == 0 {
		return nil
	}

	// Find the service port that maps to this container port
	servicePort := hc.findServicePort(service, containerPort)
	if servicePort == 0 {
		return nil
	}

	scheme := "http"
	if httpGet.Scheme == corev1.URISchemeHTTPS {
		scheme = "https"
	}

	interval := defaultCheckInterval
	if probe.PeriodSeconds != 0 {
		interval = time.Duration(probe.PeriodSeconds) * time.Second
	}

	timeout := defaultRequestTimeout
	if probe.TimeoutSeconds != 0 {
		timeout = time.Duration(probe.TimeoutSeconds) * time.Second
	}

	return &ProbeInfo{
		ServiceName:   service.Name,
		Namespace:     service.Namespace,
		PodName:       pod.Name,
		ContainerName: container.Name,
		Port:          containerPort,
		Path:          httpGet.Path,
		Scheme:        scheme,
		ServicePort:   servicePort,
		Interval:      interval,
		Timeout:       timeout,
	}
}

// resolveContainerPort resolves the container port from probe configuration
func (hc *HealthChecker) resolveContainerPort(container corev1.Container, port intstr.IntOrString) int32 {
	if port.Type == intstr.Int {
		return port.IntVal
	}

	// Resolve named port
	for _, containerPort := range container.Ports {
		if containerPort.Name == port.StrVal {
			return containerPort.ContainerPort
		}
	}

	return 0
}

// findServicePort finds the service port that maps to the given container port
func (hc *HealthChecker) findServicePort(service corev1.Service, containerPort int32) int32 {
	for _, servicePort := range service.Spec.Ports {
		if servicePort.TargetPort.Type == intstr.Int && servicePort.TargetPort.IntVal == containerPort {
			return servicePort.Port
		}
		// For named ports, we'd need to check if the name matches
		// but for simplicity, we'll use the first matching numeric port
	}
	return 0
}

// checkServiceProbe performs health check using the probe information
func (hc *HealthChecker) checkServiceProbe(ctx context.Context, serviceKey string, probe ProbeInfo) {
	logger := log.FromContext(ctx)

	// Build the health check URL using service DNS name and service port
	healthURL := fmt.Sprintf("%s://%s.%s.svc.cluster.local:%d%s", 
		probe.Scheme, probe.ServiceName, probe.Namespace, probe.ServicePort, probe.Path)

	startTime := time.Now()
	status, responseCode, err := hc.performHTTPCheck(ctx, healthURL, probe.Timeout)
	latency := time.Since(startTime)

	entry := types.HealthCheckEntry{
		Timestamp:    startTime,
		Status:       status,
		Latency:      latency,
		URL:          healthURL,
		Method:       "GET",
		ResponseCode: responseCode,
	}
	if err != nil {
		entry.Error = err.Error()
	}

	hc.updateHealthData(serviceKey, probe.ServiceName, probe.Namespace, healthURL, entry)

	logger.V(1).Info("Health check completed", 
		"service", probe.ServiceName, 
		"namespace", probe.Namespace,
		"url", healthURL,
		"status", status,
		"latency", latency,
		"responseCode", responseCode)
}

// performHTTPCheck executes the HTTP health check with specified timeout
func (hc *HealthChecker) performHTTPCheck(ctx context.Context, healthURL string, timeout time.Duration) (types.HealthStatus, int, error) {
	client := &http.Client{Timeout: timeout}
	
	req, err := http.NewRequestWithContext(ctx, "GET", healthURL, nil)
	if err != nil {
		return types.HealthStatusUnhealthy, 0, fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := client.Do(req)
	if err != nil {
		if urlErr, ok := err.(*url.Error); ok && urlErr.Timeout() {
			return types.HealthStatusUnhealthy, 0, fmt.Errorf("request timeout: %w", err)
		}
		return types.HealthStatusUnhealthy, 0, fmt.Errorf("request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode == http.StatusOK {
		return types.HealthStatusHealthy, resp.StatusCode, nil
	}

	return types.HealthStatusUnhealthy, resp.StatusCode, fmt.Errorf("unhealthy status code: %d", resp.StatusCode)
}

// updateHealthData updates the health information for a service
func (hc *HealthChecker) updateHealthData(serviceKey, serviceName, namespace, url string, entry types.HealthCheckEntry) {
	hc.mu.Lock()
	defer hc.mu.Unlock()

	healthInfo, exists := hc.healthData[serviceKey]
	if !exists {
		healthInfo = &types.ServiceHealthInfo{
			ServiceName: serviceName,
			Namespace:   namespace,
			URL:         url,
			Status:      types.HealthStatusUnknown,
			History:     make([]types.HealthCheckEntry, 0, maxHistorySize),
		}
		hc.healthData[serviceKey] = healthInfo
	}

	healthInfo.LastCheck = entry.Timestamp
	healthInfo.Status = entry.Status
	healthInfo.History = append(healthInfo.History, entry)

	if len(healthInfo.History) > maxHistorySize {
		healthInfo.History = healthInfo.History[len(healthInfo.History)-maxHistorySize:]
	}

	healthInfo.Uptime = hc.calculateUptime(healthInfo.History)

	hc.stateManager.UpdateServiceHealth(serviceName, namespace, healthInfo)
}

// calculateUptime calculates the uptime percentage from history
func (hc *HealthChecker) calculateUptime(history []types.HealthCheckEntry) float64 {
	if len(history) == 0 {
		return 0.0
	}

	healthyCount := 0
	for _, entry := range history {
		if entry.Status == types.HealthStatusHealthy {
			healthyCount++
		}
	}

	return float64(healthyCount) / float64(len(history)) * 100.0
}

// shouldIgnoreHealthCheck checks if health checking should be ignored for a service
func shouldIgnoreHealthCheck(annotations map[string]string) bool {
	return getAnnotationValue(annotations, healthIgnoreAnnotation) == "true"
}

// SetupWithManager sets up the health checker with the manager
func (hc *HealthChecker) SetupWithManager(mgr ctrl.Manager) error {
	return mgr.Add(hc)
}

// Reconcile implements manager.Runnable interface (no-op for health checker)
func (hc *HealthChecker) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	return ctrl.Result{}, nil
}