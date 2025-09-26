package controller

import (
	"context"

	corev1 "k8s.io/api/core/v1"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
)

// PodReconciler reconciles Pod objects
type PodReconciler struct {
	BaseReconciler
}

// NewPodReconciler creates a new PodReconciler
func NewPodReconciler(mgr ctrl.Manager, stateManager *StateManager) *PodReconciler {
	return &PodReconciler{
		BaseReconciler: newBaseReconciler(mgr, stateManager),
	}
}

// Reconcile handles Pod events
func (r *PodReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var pod corev1.Pod
	if err := r.Get(ctx, req.NamespacedName, &pod); err != nil {
		if client.IgnoreNotFound(err) == nil {
			r.stateManager.DeletePod(ctx, req.Name, req.Namespace)
			return ctrl.Result{}, nil
		}
		logger.Error(err, "Failed to get pod")
		return ctrl.Result{}, err
	}

	r.stateManager.UpdatePod(ctx, pod)

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager
func (r *PodReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Pod{}).
		Complete(r)
}