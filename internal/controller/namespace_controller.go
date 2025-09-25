package controller

import (
	"context"

	corev1 "k8s.io/api/core/v1"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
)

// NamespaceReconciler reconciles Namespace objects
type NamespaceReconciler struct {
	BaseReconciler
}

// NewNamespaceReconciler creates a new NamespaceReconciler
func NewNamespaceReconciler(mgr ctrl.Manager, stateManager *StateManager) *NamespaceReconciler {
	return &NamespaceReconciler{
		BaseReconciler: newBaseReconciler(mgr, stateManager),
	}
}

// Reconcile handles Namespace events
func (r *NamespaceReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var namespace corev1.Namespace
	if err := r.Get(ctx, req.NamespacedName, &namespace); err != nil {
		if client.IgnoreNotFound(err) == nil {
			r.stateManager.removeNamespace(req.Name)
			r.stateManager.broadcastUpdate()
			return ctrl.Result{}, nil
		}
		logger.Error(err, "Failed to get namespace")
		return ctrl.Result{}, err
	}

	r.stateManager.updateNamespace(&namespace)
	r.stateManager.broadcastUpdate()

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager
func (r *NamespaceReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Namespace{}).
		Complete(r)
}