package controller

import (
	"context"
	"fmt"

	"github.com/kdwils/constellation/internal/healthcheck"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
)

// PodReconciler reconciles Pod objects
type PodReconciler struct {
	client.Client
	Scheme        *runtime.Scheme
	HealthChecker *healthcheck.HealthChecker
}

// NewPodReconciler creates a new PodReconciler
func NewPodReconciler(mgr ctrl.Manager, healthChecker *healthcheck.HealthChecker) *PodReconciler {
	return &PodReconciler{
		Client:        mgr.GetClient(),
		Scheme:        mgr.GetScheme(),
		HealthChecker: healthChecker,
	}
}

// +kubebuilder:rbac:groups="",resources=pods,verbs=get;list;watch
// +kubebuilder:rbac:groups="",resources=services,verbs=get;list;watch

// Reconcile handles Pod events
func (r *PodReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var pod corev1.Pod
	if err := r.Get(ctx, req.NamespacedName, &pod); err != nil {
		if client.IgnoreNotFound(err) != nil {
			logger.Error(err, "failed to get pod")
			return ctrl.Result{}, err
		}
	}

	var services corev1.ServiceList
	if err := r.List(ctx, &services, client.InNamespace(req.Namespace)); err != nil {
		logger.Error(err, "failed to list services")
		return ctrl.Result{}, err
	}

	var pods corev1.PodList
	if err := r.List(ctx, &pods, client.InNamespace(req.Namespace)); err != nil {
		logger.Error(err, "failed to list pods")
		return ctrl.Result{}, err
	}

	for _, service := range services.Items {
		if shouldIgnoreResource(service.Annotations) {
			continue
		}

		if !labelsMatch(service.Spec.Selector, pod.Labels) {
			continue
		}

		checks := extractHealthChecksFromPods(service, pods.Items)
		if len(checks) > 0 {
			serviceKey := fmt.Sprintf("%s/%s", service.Namespace, service.Name)
			logger.Info("updating health check from pod change", "service", serviceKey, "pod", req.Name, "checks", len(checks))
			r.HealthChecker.RegisterHealthTarget(serviceKey, checks)
		}
	}

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager
func (r *PodReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Pod{}).
		Named("pod").
		Complete(r)
}
