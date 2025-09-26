package controller

import (
	"context"

	corev1 "k8s.io/api/core/v1"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
	gatewayv1beta1 "sigs.k8s.io/gateway-api/apis/v1beta1"
)

// HTTPRouteReconciler reconciles HTTPRoute objects
type HTTPRouteReconciler struct {
	BaseReconciler
}

// NewHTTPRouteReconciler creates a new HTTPRouteReconciler
func NewHTTPRouteReconciler(mgr ctrl.Manager, stateManager *StateManager) *HTTPRouteReconciler {
	return &HTTPRouteReconciler{
		BaseReconciler: newBaseReconciler(mgr, stateManager),
	}
}

// Reconcile handles HTTPRoute events
func (r *HTTPRouteReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var httpRoute gatewayv1beta1.HTTPRoute
	if err := r.Get(ctx, req.NamespacedName, &httpRoute); err != nil {
		if client.IgnoreNotFound(err) == nil {
			r.stateManager.DeleteHTTPRoute(ctx, req.Name, req.Namespace)
			return ctrl.Result{}, nil
		}
		logger.Error(err, "Failed to get httproute")
		return ctrl.Result{}, err
	}

	var services corev1.ServiceList
	if err := r.List(ctx, &services, client.InNamespace(req.Namespace)); err != nil {
		logger.Error(err, "Failed to list services")
		return ctrl.Result{}, err
	}

	var pods corev1.PodList
	if err := r.List(ctx, &pods, client.InNamespace(req.Namespace)); err != nil {
		logger.Error(err, "Failed to list pods")
		return ctrl.Result{}, err
	}

	r.stateManager.UpdateHTTPRoute(ctx, httpRoute, services.Items, pods.Items)

	return ctrl.Result{}, nil
}

// SetupWithManager sets up the controller with the Manager
func (r *HTTPRouteReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&gatewayv1beta1.HTTPRoute{}).
		Complete(r)
}