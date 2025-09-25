package controller

import (
	"context"

	corev1 "k8s.io/api/core/v1"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
)

// ServiceReconciler reconciles Service objects
type ServiceReconciler struct {
	BaseReconciler
}

// NewServiceReconciler creates a new ServiceReconciler
func NewServiceReconciler(mgr ctrl.Manager, stateManager *StateManager) *ServiceReconciler {
	return &ServiceReconciler{
		BaseReconciler: newBaseReconciler(mgr, stateManager),
	}
}

// Reconcile handles Service events
func (r *ServiceReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var service corev1.Service
	if err := r.Get(ctx, req.NamespacedName, &service); err != nil {
		if client.IgnoreNotFound(err) == nil {
			r.stateManager.removeService(req.Name, req.Namespace)
			r.stateManager.broadcastUpdate()
			return ctrl.Result{}, nil
		}
		logger.Error(err, "Failed to get service")
		return ctrl.Result{}, err
	}

	var pods corev1.PodList
	if err := r.List(ctx, &pods, client.InNamespace(req.Namespace)); err != nil {
		logger.Error(err, "Failed to list pods")
		return ctrl.Result{}, err
	}

	r.stateManager.updateService(&service, pods.Items)
	r.stateManager.broadcastUpdate()

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager
func (r *ServiceReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Service{}).
		Complete(r)
}