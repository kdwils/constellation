/*
Copyright 2025.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package controller

import (
	"context"
	"fmt"

	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	logf "sigs.k8s.io/controller-runtime/pkg/log"

	healthv1alpha1 "github.com/kdwils/constellation/api/v1alpha1"
	"github.com/kdwils/constellation/internal/healthcheck"
)

// HealthCheckReconciler reconciles a HealthCheck object
type HealthCheckReconciler struct {
	client.Client
	Scheme        *runtime.Scheme
	HealthChecker *healthcheck.HealthChecker
}

// +kubebuilder:rbac:groups=health.kyledev.co,resources=healthchecks,verbs=get;list;watch;create;update;patch;delete
// +kubebuilder:rbac:groups=health.kyledev.co,resources=healthchecks/status,verbs=get;update;patch
// +kubebuilder:rbac:groups=health.kyledev.co,resources=healthchecks/finalizers,verbs=update

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
func (r *HealthCheckReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := logf.FromContext(ctx)

	var healthCheck healthv1alpha1.HealthCheck
	if err := r.Get(ctx, req.NamespacedName, &healthCheck); err != nil {
		return ctrl.Result{}, client.IgnoreNotFound(err)
	}

	serviceKey := fmt.Sprintf("%s/%s", healthCheck.ObjectMeta.Namespace, healthCheck.ObjectMeta.Name)

	if !healthCheck.DeletionTimestamp.IsZero() {
		logger.Info("unregistering health check", "service", serviceKey)
		r.HealthChecker.UnregisterHealthTarget(serviceKey)
		return ctrl.Result{}, nil
	}

	checks := convertToCheckConfigs(healthCheck.Spec.Checks)

	logger.Info("registering custom health check", "identifier", serviceKey, "checks", len(checks))
	r.HealthChecker.RegisterHealthTarget(serviceKey, checks)

	return ctrl.Result{}, nil
}

func convertToCheckConfigs(apiChecks []healthv1alpha1.CheckConfig) []healthcheck.CheckConfig {
	checks := make([]healthcheck.CheckConfig, len(apiChecks))
	for i, apiCheck := range apiChecks {
		checks[i] = healthcheck.CheckConfig{
			Name:     apiCheck.Name,
			URL:      apiCheck.URL,
			Interval: apiCheck.Interval.Duration,
			Timeout:  apiCheck.Timeout.Duration,
			Protocol: apiCheck.Protocol,
		}
	}
	return checks
}

// SetupWithManager sets up the controller with the Manager.
func (r *HealthCheckReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&healthv1alpha1.HealthCheck{}).
		Named("healthcheck").
		Complete(r)
}
