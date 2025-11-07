package controller

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/kdwils/constellation/internal/healthcheck"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
)

const ignoreAnnotation = "constellation.kyledev.co/ignore"

// ServiceReconciler reconciles Service objects
type ServiceReconciler struct {
	client.Client
	Scheme        *runtime.Scheme
	HealthChecker *healthcheck.HealthChecker
}

// NewServiceReconciler creates a new ServiceReconciler
func NewServiceReconciler(mgr ctrl.Manager, healthChecker *healthcheck.HealthChecker) *ServiceReconciler {
	return &ServiceReconciler{
		Client:        mgr.GetClient(),
		Scheme:        mgr.GetScheme(),
		HealthChecker: healthChecker,
	}
}

// +kubebuilder:rbac:groups="",resources=services,verbs=get;list;watch
// +kubebuilder:rbac:groups="",resources=pods,verbs=get;list;watch

// Reconcile handles Service events
func (r *ServiceReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var service corev1.Service
	if err := r.Get(ctx, req.NamespacedName, &service); err != nil {
		if client.IgnoreNotFound(err) == nil {
			serviceKey := fmt.Sprintf("%s/%s", req.Namespace, req.Name)
			logger.Info("service deleted, unregistering health check", "service", serviceKey)
			r.HealthChecker.UnregisterHealthTarget(serviceKey)
			return ctrl.Result{}, nil
		}
		logger.Error(err, "failed to get service")
		return ctrl.Result{}, err
	}

	if shouldIgnoreResource(service.Annotations) {
		return ctrl.Result{}, nil
	}

	var pods corev1.PodList
	if err := r.List(ctx, &pods, client.InNamespace(req.Namespace)); err != nil {
		logger.Error(err, "failed to list pods")
		return ctrl.Result{}, err
	}

	checks := extractHealthChecksFromPods(service, pods.Items)
	if len(checks) > 0 {
		serviceKey := fmt.Sprintf("%s/%s", service.Namespace, service.Name)
		logger.Info("registering discovered service health check", "identifier", serviceKey, "checks", len(checks))
		r.HealthChecker.RegisterHealthTarget(serviceKey, checks)
	}

	return ctrl.Result{}, nil
}

// extractHealthChecksFromPods extracts health check configurations from pod liveness probes
func extractHealthChecksFromPods(service corev1.Service, pods []corev1.Pod) []healthcheck.CheckConfig {
	checkName := fmt.Sprintf("%s/%s", service.Namespace, service.Name)
	var checks []healthcheck.CheckConfig
	seenURLs := make(map[string]bool)

	for _, pod := range pods {
		if shouldIgnoreResource(pod.Annotations) {
			continue
		}
		if pod.Namespace != service.Namespace {
			continue
		}
		if !labelsMatch(service.Spec.Selector, pod.Labels) {
			continue
		}
		if !shouldIncludePod(pod) {
			continue
		}

		for _, container := range pod.Spec.Containers {
			if container.LivenessProbe == nil {
				continue
			}
			if container.LivenessProbe.HTTPGet == nil {
				continue
			}

			probe := container.LivenessProbe
			httpGet := probe.HTTPGet

			if probe.PeriodSeconds == 0 || probe.TimeoutSeconds == 0 {
				continue
			}

			containerPort := httpGet.Port.IntVal
			if containerPort == 0 {
				containerPort = resolveNamedPort(httpGet.Port.StrVal, container.Ports)
			}
			if containerPort == 0 {
				continue
			}

			servicePort := findServicePortForContainer(service, containerPort)
			if servicePort == 0 {
				continue
			}

			scheme := strings.ToLower(string(httpGet.Scheme))
			url := fmt.Sprintf("%s://%s.%s.svc.cluster.local:%d%s", scheme, service.Name, service.Namespace, servicePort, httpGet.Path)

			if seenURLs[url] {
				continue
			}
			seenURLs[url] = true

			checks = append(checks, healthcheck.CheckConfig{
				Name:     checkName,
				URL:      url,
				Interval: time.Duration(probe.PeriodSeconds) * time.Second,
				Timeout:  time.Duration(probe.TimeoutSeconds) * time.Second,
				Protocol: scheme,
			})
		}
	}

	return checks
}

// resolveNamedPort resolves a named port to its numeric value
func resolveNamedPort(portName string, ports []corev1.ContainerPort) int32 {
	if portName == "" {
		return 0
	}
	for _, port := range ports {
		if port.Name == portName {
			return port.ContainerPort
		}
	}
	return 0
}

// findServicePortForContainer finds the service port that maps to a container port
func findServicePortForContainer(service corev1.Service, containerPort int32) int32 {
	for _, port := range service.Spec.Ports {
		if port.TargetPort.IntVal == containerPort {
			return port.Port
		}
		if port.TargetPort.StrVal != "" {
			continue
		}
		if port.TargetPort.IntVal == 0 && port.Port == containerPort {
			return port.Port
		}
	}
	return 0
}

// shouldIgnoreResource checks if a resource should be ignored
func shouldIgnoreResource(annotations map[string]string) bool {
	if annotations == nil {
		return false
	}
	value, exists := annotations[ignoreAnnotation]
	if !exists {
		return false
	}
	return value == "true"
}

// labelsMatch checks if selector matches labels
func labelsMatch(selector, labels map[string]string) bool {
	if len(selector) == 0 {
		return false
	}
	for key, value := range selector {
		if labels[key] != value {
			return false
		}
	}
	return true
}

// shouldIncludePod checks if a pod should be included
func shouldIncludePod(pod corev1.Pod) bool {
	if pod.Status.Phase != corev1.PodRunning && pod.Status.Phase != corev1.PodPending {
		return false
	}
	if pod.DeletionTimestamp != nil {
		return false
	}
	return true
}

// SetupWithManager sets up the controller with the Manager
func (r *ServiceReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Service{}).
		Named("service").
		Complete(r)
}
