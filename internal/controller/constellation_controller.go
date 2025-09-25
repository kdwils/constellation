package controller

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"slices"
	"sync"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
	gatewayv1beta1 "sigs.k8s.io/gateway-api/apis/v1beta1"

	"github.com/kdwils/constellation/internal/types"
)

// StateManager maintains the hierarchy state and serves it via HTTP
type StateManager struct {
	hierarchy   []types.HierarchyNode
	mu          sync.RWMutex
	subscribers map[chan []types.HierarchyNode]bool
	subMu       sync.RWMutex
	updateChan  chan bool
}

// NewStateManager creates a new state manager
func NewStateManager(updateChan chan bool) *StateManager {
	return &StateManager{
		hierarchy:   make([]types.HierarchyNode, 0),
		subscribers: make(map[chan []types.HierarchyNode]bool),
		updateChan:  updateChan,
	}
}

// BaseReconciler provides common functionality for all resource reconcilers
type BaseReconciler struct {
	client.Client
	Scheme       *runtime.Scheme
	stateManager *StateManager
}

// NamespaceReconciler reconciles Namespace objects
type NamespaceReconciler struct {
	BaseReconciler
}

// ServiceReconciler reconciles Service objects
type ServiceReconciler struct {
	BaseReconciler
}

// PodReconciler reconciles Pod objects
type PodReconciler struct {
	BaseReconciler
}

// HTTPRouteReconciler reconciles HTTPRoute objects
type HTTPRouteReconciler struct {
	BaseReconciler
}

// newBaseReconciler creates a new BaseReconciler with common initialization
func newBaseReconciler(mgr ctrl.Manager, stateManager *StateManager) BaseReconciler {
	return BaseReconciler{
		Client:       mgr.GetClient(),
		Scheme:       mgr.GetScheme(),
		stateManager: stateManager,
	}
}

func NewNamespaceReconciler(mgr ctrl.Manager, stateManager *StateManager) *NamespaceReconciler {
	return &NamespaceReconciler{
		BaseReconciler: newBaseReconciler(mgr, stateManager),
	}
}

func NewServiceReconciler(mgr ctrl.Manager, stateManager *StateManager) *ServiceReconciler {
	return &ServiceReconciler{
		BaseReconciler: newBaseReconciler(mgr, stateManager),
	}
}

func NewPodReconciler(mgr ctrl.Manager, stateManager *StateManager) *PodReconciler {
	return &PodReconciler{
		BaseReconciler: newBaseReconciler(mgr, stateManager),
	}
}

func NewHTTPRouteReconciler(mgr ctrl.Manager, stateManager *StateManager) *HTTPRouteReconciler {
	return &HTTPRouteReconciler{
		BaseReconciler: newBaseReconciler(mgr, stateManager),
	}
}

// SetupWithManager sets up each controller with the Manager
func (r *NamespaceReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Namespace{}).
		Complete(r)
}

func (r *ServiceReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Service{}).
		Complete(r)
}

func (r *PodReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&corev1.Pod{}).
		Complete(r)
}

func (r *HTTPRouteReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&gatewayv1beta1.HTTPRoute{}).
		Complete(r)
}

// Namespace reconciler
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

// Service reconciler
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

func (r *PodReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var pod corev1.Pod
	if err := r.Get(ctx, req.NamespacedName, &pod); err != nil {
		if client.IgnoreNotFound(err) == nil {
			r.stateManager.removePod(req.Name, req.Namespace)
			r.stateManager.broadcastUpdate()
			return ctrl.Result{}, nil
		}
		logger.Error(err, "Failed to get pod")
		return ctrl.Result{}, err
	}

	r.stateManager.updatePod(&pod)
	r.stateManager.broadcastUpdate()

	return ctrl.Result{}, nil
}

// HTTPRoute reconciler
func (r *HTTPRouteReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	logger := log.FromContext(ctx)

	var httpRoute gatewayv1beta1.HTTPRoute
	if err := r.Get(ctx, req.NamespacedName, &httpRoute); err != nil {
		if client.IgnoreNotFound(err) == nil {
			r.stateManager.removeHTTPRoute(req.Name, req.Namespace)
			r.stateManager.broadcastUpdate()
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

	r.stateManager.updateHTTPRoute(&httpRoute, services.Items, pods.Items)
	r.stateManager.broadcastUpdate()

	return ctrl.Result{}, nil
}

func labelsMatch(selectors, labels map[string]string) bool {
	if selectors == nil || labels == nil {
		return false
	}

	for key, value := range selectors {
		if labels[key] != value {
			return false
		}
	}
	return true
}

// GetHierarchy returns the current hierarchy state
func (sm *StateManager) GetHierarchy() []types.HierarchyNode {
	sm.mu.RLock()
	defer sm.mu.RUnlock()
	return append([]types.HierarchyNode(nil), sm.hierarchy...)
}

// Subscribe adds a channel to receive state updates
func (sm *StateManager) Subscribe() chan []types.HierarchyNode {
	sm.subMu.Lock()
	defer sm.subMu.Unlock()

	ch := make(chan []types.HierarchyNode, 10)
	sm.subscribers[ch] = true
	return ch
}

// Unsubscribe removes a channel from receiving state updates
func (sm *StateManager) Unsubscribe(ch chan []types.HierarchyNode) {
	sm.subMu.Lock()
	defer sm.subMu.Unlock()

	delete(sm.subscribers, ch)
	close(ch)
}

// broadcastUpdate sends state to all subscribers and legacy channel
func (sm *StateManager) broadcastUpdate() {
	sm.mu.RLock()
	currentState := make([]types.HierarchyNode, len(sm.hierarchy))
	copy(currentState, sm.hierarchy)
	sm.mu.RUnlock()

	sm.subMu.RLock()
	defer sm.subMu.RUnlock()

	for subscriber := range sm.subscribers {
		subscriber <- currentState
	}

	sm.updateChan <- true
}

// Helper functions to check if a resource should be included
func shouldIncludePod(pod *corev1.Pod) bool {
	if pod.DeletionTimestamp != nil {
		return false
	}

	if pod.Status.Phase == corev1.PodFailed {
		return false
	}

	if pod.Status.Phase == corev1.PodSucceeded {
		return false
	}

	return true
}

func shouldIncludeService(_ *corev1.Service) bool {
	return true
}

func shouldIncludeHTTPRoute(_ *gatewayv1beta1.HTTPRoute) bool {
	return true
}

// removeNodeByKind removes a node from the hierarchy by kind, name, and namespace
func removeNodeByKind(node *types.HierarchyNode, kind types.ResourceKind, name, namespace string) {
	node.Relatives = slices.DeleteFunc(node.Relatives, func(child types.HierarchyNode) bool {
		return child.Kind == kind && child.Name == name && (child.Namespace == nil || *child.Namespace == namespace)
	})

	for i := range node.Relatives {
		removeNodeByKind(&node.Relatives[i], kind, name, namespace)
	}
}

// Convert Kubernetes resources to HierarchyNode
func namespaceToHierarchyNode(ns *corev1.Namespace) types.HierarchyNode {
	return types.HierarchyNode{
		Kind:      types.ResourceKindNamespace,
		Name:      ns.Name,
		Namespace: nil,
		Relatives: []types.HierarchyNode{},
		Labels:    ns.Labels,
	}
}

func serviceToHierarchyNode(svc *corev1.Service) types.HierarchyNode {
	metadata := extractServiceMetadata(svc)
	var namespace *string
	if svc.Namespace != "" {
		namespace = &svc.Namespace
	}

	return types.HierarchyNode{
		Kind:            types.ResourceKindService,
		Name:            svc.Name,
		Namespace:       namespace,
		Relatives:       []types.HierarchyNode{},
		Hostnames:       metadata.Hostnames,
		Selectors:       metadata.Selectors,
		Ports:           metadata.Ports,
		PortMappings:    metadata.PortMappings,
		TargetPorts:     metadata.TargetPorts,
		TargetPortNames: metadata.TargetPortNames,
		ContainerPorts:  metadata.ContainerPorts,
		Labels:          metadata.Labels,
		Phase:           metadata.Phase,
		BackendRefs:     metadata.BackendRefs,
		ServiceType:     metadata.ServiceType,
		ClusterIPs:      metadata.ClusterIPs,
		ExternalIPs:     metadata.ExternalIPs,
		PodIPs:          metadata.PodIPs,
		Group:           metadata.Group,
		DisplayName:     metadata.DisplayName,
		Ignore:          metadata.Ignore,
	}
}

func podToHierarchyNode(pod *corev1.Pod) types.HierarchyNode {
	metadata := extractPodMetadata(pod)
	var namespace *string
	if pod.Namespace != "" {
		namespace = &pod.Namespace
	}

	return types.HierarchyNode{
		Kind:            types.ResourceKindPod,
		Name:            pod.Name,
		Namespace:       namespace,
		Relatives:       []types.HierarchyNode{},
		Hostnames:       metadata.Hostnames,
		Selectors:       metadata.Selectors,
		Ports:           metadata.Ports,
		PortMappings:    metadata.PortMappings,
		TargetPorts:     metadata.TargetPorts,
		TargetPortNames: metadata.TargetPortNames,
		ContainerPorts:  metadata.ContainerPorts,
		Labels:          metadata.Labels,
		Phase:           metadata.Phase,
		BackendRefs:     metadata.BackendRefs,
		ServiceType:     metadata.ServiceType,
		ClusterIPs:      metadata.ClusterIPs,
		ExternalIPs:     metadata.ExternalIPs,
		PodIPs:          metadata.PodIPs,
		Group:           metadata.Group,
		DisplayName:     metadata.DisplayName,
		Ignore:          metadata.Ignore,
	}
}

func httpRouteToHierarchyNode(route *gatewayv1beta1.HTTPRoute) types.HierarchyNode {
	metadata := extractHTTPRouteMetadata(route)
	var namespace *string
	if route.Namespace != "" {
		namespace = &route.Namespace
	}

	return types.HierarchyNode{
		Kind:            types.ResourceKindHTTPRoute,
		Name:            route.Name,
		Namespace:       namespace,
		Relatives:       []types.HierarchyNode{},
		Hostnames:       metadata.Hostnames,
		Selectors:       metadata.Selectors,
		Ports:           metadata.Ports,
		PortMappings:    metadata.PortMappings,
		TargetPorts:     metadata.TargetPorts,
		TargetPortNames: metadata.TargetPortNames,
		ContainerPorts:  metadata.ContainerPorts,
		Labels:          metadata.Labels,
		Phase:           metadata.Phase,
		BackendRefs:     metadata.BackendRefs,
		ServiceType:     metadata.ServiceType,
		ClusterIPs:      metadata.ClusterIPs,
		ExternalIPs:     metadata.ExternalIPs,
		PodIPs:          metadata.PodIPs,
		Group:           metadata.Group,
		DisplayName:     metadata.DisplayName,
		Ignore:          metadata.Ignore,
	}
}

// ServeHTTP implements http.Handler to serve cluster state as JSON
func (sm *StateManager) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	hierarchy := sm.GetHierarchy()

	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(hierarchy); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
}

// extractGroupFromAnnotations extracts group information from constellation annotations
func extractGroupFromAnnotations(annotations map[string]string) string {
	if annotations == nil {
		return ""
	}
	return annotations["constellation.kyledev.co/group"]
}

// extractDisplayNameFromAnnotations extracts display name from constellation annotations
func extractDisplayNameFromAnnotations(annotations map[string]string) string {
	if annotations == nil {
		return ""
	}
	return annotations["constellation.kyledev.co/display-name"]
}

// shouldIgnoreResource checks if resource should be ignored based on constellation annotations
func shouldIgnoreResource(annotations map[string]string) bool {
	if annotations == nil {
		return false
	}
	return annotations["constellation.kyledev.co/ignore"] == "true"
}

func extractServiceMetadata(svc *corev1.Service) types.ResourceMetadata {
	metadata := types.ResourceMetadata{
		Labels:    svc.Labels,
		Selectors: svc.Spec.Selector,
	}

	metadata.Group = extractGroupFromAnnotations(svc.Annotations)
	metadata.DisplayName = extractDisplayNameFromAnnotations(svc.Annotations)
	metadata.Ignore = shouldIgnoreResource(svc.Annotations)

	if svc.Spec.Type != "" {
		serviceType := string(svc.Spec.Type)
		metadata.ServiceType = &serviceType
	}

	if len(svc.Spec.ClusterIPs) > 0 {
		metadata.ClusterIPs = svc.Spec.ClusterIPs
	}

	if len(svc.Spec.ExternalIPs) > 0 {
		metadata.ExternalIPs = svc.Spec.ExternalIPs
	}

	var ports []int32
	var portMappings []string
	var targetPorts []int32
	var targetPortNames []string

	for _, port := range svc.Spec.Ports {
		ports = append(ports, port.Port)
		portMappings = append(portMappings, port.TargetPort.String())

		if port.TargetPort.IntVal != 0 {
			targetPorts = append(targetPorts, port.TargetPort.IntVal)
		}
		if port.TargetPort.StrVal != "" {
			targetPortNames = append(targetPortNames, port.TargetPort.StrVal)
		}
	}

	metadata.Ports = ports
	metadata.PortMappings = portMappings
	metadata.TargetPorts = targetPorts
	metadata.TargetPortNames = targetPortNames

	return metadata
}

func extractPodMetadata(pod *corev1.Pod) types.ResourceMetadata {
	metadata := types.ResourceMetadata{
		Labels:      pod.Labels,
		Group:       extractGroupFromAnnotations(pod.Annotations),
		DisplayName: extractDisplayNameFromAnnotations(pod.Annotations),
		Ignore:      shouldIgnoreResource(pod.Annotations),
	}

	if pod.Status.Phase != "" {
		phase := string(pod.Status.Phase)
		metadata.Phase = &phase
	}

	var podIPs []string
	for _, ip := range pod.Status.PodIPs {
		podIPs = append(podIPs, ip.IP)
	}
	if len(podIPs) > 0 {
		metadata.PodIPs = podIPs
	}

	var containerPorts []types.ContainerPortInfo
	for _, container := range pod.Spec.Containers {
		for _, port := range container.Ports {
			portInfo := types.ContainerPortInfo{
				Port: port.ContainerPort,
			}
			if port.Name != "" {
				portInfo.Name = &port.Name
			}
			if port.Protocol != "" {
				protocol := string(port.Protocol)
				portInfo.Protocol = &protocol
			}
			containerPorts = append(containerPorts, portInfo)
		}
	}
	if len(containerPorts) > 0 {
		metadata.ContainerPorts = containerPorts
	}

	return metadata
}

func extractHTTPRouteMetadata(route *gatewayv1beta1.HTTPRoute) types.ResourceMetadata {
	metadata := types.ResourceMetadata{
		Labels:      route.Labels,
		Group:       extractGroupFromAnnotations(route.Annotations),
		DisplayName: extractDisplayNameFromAnnotations(route.Annotations),
		Ignore:      shouldIgnoreResource(route.Annotations),
	}

	var hostnames []string
	for _, hostname := range route.Spec.Hostnames {
		hostnames = append(hostnames, string(hostname))
	}
	if len(hostnames) > 0 {
		metadata.Hostnames = hostnames
	}

	var backendRefs []string
	for _, rule := range route.Spec.Rules {
		for _, backendRef := range rule.BackendRefs {
			if backendRef.Name != "" {
				backendRefs = append(backendRefs, string(backendRef.Name))
			}
		}
	}
	if len(backendRefs) > 0 {
		metadata.BackendRefs = backendRefs
	}

	return metadata
}

// StateManager update methods following the Rust implementation pattern

func (sm *StateManager) updateNamespace(namespace *corev1.Namespace) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	// Find existing namespace or create new one
	var nsNode *types.HierarchyNode
	for i := range sm.hierarchy {
		if sm.hierarchy[i].Kind == types.ResourceKindNamespace && sm.hierarchy[i].Name == namespace.Name {
			nsNode = &sm.hierarchy[i]
			break
		}
	}

	if nsNode == nil {
		newNode := namespaceToHierarchyNode(namespace)
		sm.hierarchy = append(sm.hierarchy, newNode)
	}

	// Sort hierarchy by name
	slices.SortFunc(sm.hierarchy, func(a, b types.HierarchyNode) int {
		if a.Name < b.Name {
			return -1
		}
		if a.Name > b.Name {
			return 1
		}
		return 0
	})
}

func (sm *StateManager) removeNamespace(name string) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	sm.hierarchy = slices.DeleteFunc(sm.hierarchy, func(node types.HierarchyNode) bool {
		return node.Kind == types.ResourceKindNamespace && node.Name == name
	})
}

func (sm *StateManager) updateService(service *corev1.Service, pods []corev1.Pod) {
	if !shouldIncludeService(service) {
		return
	}

	sm.mu.Lock()
	defer sm.mu.Unlock()

	serviceName := service.Name
	serviceNamespace := service.Namespace

	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindService, serviceName, serviceNamespace)
	}

	var namespaceNode *types.HierarchyNode
	for i := range sm.hierarchy {
		if sm.hierarchy[i].Kind == types.ResourceKindNamespace && sm.hierarchy[i].Name == serviceNamespace {
			namespaceNode = &sm.hierarchy[i]
			break
		}
	}

	if namespaceNode == nil {
		return
	}

	httpRouteIndex := findHTTPRouteForService(*namespaceNode, serviceName)
	serviceNode := createServiceNodeWithPods(service, pods)

	if httpRouteIndex != -1 {
		namespaceNode.Relatives[httpRouteIndex].Relatives = append(namespaceNode.Relatives[httpRouteIndex].Relatives, serviceNode)
		return
	}

	namespaceNode.Relatives = append(namespaceNode.Relatives, serviceNode)
}

func (sm *StateManager) removeService(name, namespace string) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindService, name, namespace)
	}
}

func (sm *StateManager) updatePod(pod *corev1.Pod) {
	podName := pod.Name
	podNamespace := pod.Namespace

	sm.mu.Lock()
	defer sm.mu.Unlock()

	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindPod, podName, podNamespace)
	}

	if !shouldIncludePod(pod) {
		return
	}

	var namespaceNode *types.HierarchyNode
	for i := range sm.hierarchy {
		if sm.hierarchy[i].Kind == types.ResourceKindNamespace && sm.hierarchy[i].Name == podNamespace {
			namespaceNode = &sm.hierarchy[i]
			break
		}
	}

	if namespaceNode == nil {
		return
	}

	podNode := podToHierarchyNode(pod)
	if !sm.addPodToMatchingServiceInNamespace(namespaceNode, pod, podNode) {
		namespaceNode.Relatives = append(namespaceNode.Relatives, podNode)
	}
}

func (sm *StateManager) removePod(name, namespace string) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindPod, name, namespace)
	}
}

func (sm *StateManager) updateHTTPRoute(route *gatewayv1beta1.HTTPRoute, services []corev1.Service, pods []corev1.Pod) {
	if !shouldIncludeHTTPRoute(route) {
		return
	}

	sm.mu.Lock()
	defer sm.mu.Unlock()

	routeName := route.Name
	routeNamespace := route.Namespace

	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindHTTPRoute, routeName, routeNamespace)
	}

	var namespaceNode *types.HierarchyNode
	for i := range sm.hierarchy {
		if sm.hierarchy[i].Kind == types.ResourceKindNamespace && sm.hierarchy[i].Name == routeNamespace {
			namespaceNode = &sm.hierarchy[i]
			break
		}
	}

	if namespaceNode == nil {
		return
	}

	routeNode := httpRouteToHierarchyNode(route)
	routeNode.Relatives = findServicesForHTTPRoute(route, services, pods)
	namespaceNode.Relatives = append(namespaceNode.Relatives, routeNode)
}

func (sm *StateManager) removeHTTPRoute(name, namespace string) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindHTTPRoute, name, namespace)
	}
}

// Pure helper functions that don't modify state

// findHTTPRouteForService returns the index of HTTPRoute that references the service, or -1
func findHTTPRouteForService(namespaceNode types.HierarchyNode, serviceName string) int {
	for i, relative := range namespaceNode.Relatives {
		if relative.Kind != types.ResourceKindHTTPRoute {
			continue
		}
		if serviceReferencedByHTTPRoute(relative, serviceName) {
			return i
		}
	}
	return -1
}

// serviceReferencedByHTTPRoute checks if a service is referenced by an HTTPRoute
func serviceReferencedByHTTPRoute(httpRoute types.HierarchyNode, serviceName string) bool {
	for _, backendRef := range httpRoute.BackendRefs {
		if backendRef == serviceName {
			return true
		}
	}
	return false
}

// findMatchingPods returns all pods that match the service selector
func findMatchingPods(service *corev1.Service, pods []corev1.Pod) []types.HierarchyNode {
	var matchingPods []types.HierarchyNode
	for _, pod := range pods {
		if pod.Namespace != service.Namespace {
			continue
		}
		if !shouldIncludePod(&pod) {
			continue
		}
		if !labelsMatch(service.Spec.Selector, pod.Labels) {
			continue
		}
		matchingPods = append(matchingPods, podToHierarchyNode(&pod))
	}
	return matchingPods
}

// createServiceNodeWithPods creates a service node with matching pods already included
func createServiceNodeWithPods(service *corev1.Service, pods []corev1.Pod) types.HierarchyNode {
	serviceNode := serviceToHierarchyNode(service)
	serviceNode.Relatives = findMatchingPods(service, pods)
	return serviceNode
}

// findServicesForHTTPRoute returns all services referenced by the HTTPRoute
func findServicesForHTTPRoute(route *gatewayv1beta1.HTTPRoute, services []corev1.Service, pods []corev1.Pod) []types.HierarchyNode {
	var serviceNodes []types.HierarchyNode
	for _, rule := range route.Spec.Rules {
		for _, backendRef := range rule.BackendRefs {
			serviceName := string(backendRef.Name)
			for _, service := range services {
				if service.Name != serviceName || service.Namespace != route.Namespace {
					continue
				}
				serviceNode := createServiceNodeWithPods(&service, pods)
				serviceNodes = append(serviceNodes, serviceNode)
				break
			}
		}
	}
	return serviceNodes
}

// podMatchesAnyService checks if a pod matches any service selector in the namespace
func podMatchesAnyService(pod *corev1.Pod, namespaceNode types.HierarchyNode) bool {
	var checkNode func(types.HierarchyNode) bool
	checkNode = func(node types.HierarchyNode) bool {
		if node.Kind == types.ResourceKindService {
			serviceNamespace := ""
			if node.Namespace != nil {
				serviceNamespace = *node.Namespace
			}
			if serviceNamespace == pod.Namespace && labelsMatch(node.Selectors, pod.Labels) {
				return true
			}
		}
		for _, relative := range node.Relatives {
			if checkNode(relative) {
				return true
			}
		}
		return false
	}
	return checkNode(namespaceNode)
}

// BuildInitialState loads all existing resources and builds the initial hierarchy
func (sm *StateManager) BuildInitialState(ctx context.Context, client client.Client) error {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	if err := sm.loadAllNamespaces(ctx, client); err != nil {
		return err
	}

	var services corev1.ServiceList
	if err := client.List(ctx, &services); err != nil {
		return fmt.Errorf("failed to list services: %w", err)
	}

	var pods corev1.PodList
	if err := client.List(ctx, &pods); err != nil {
		return fmt.Errorf("failed to list pods: %w", err)
	}

	var httpRoutes gatewayv1beta1.HTTPRouteList
	client.List(ctx, &httpRoutes) // Ignore errors - HTTPRoutes might not be available

	sm.addHTTPRoutesToHierarchy(httpRoutes.Items, services.Items, pods.Items)
	sm.addServicesToHierarchy(services.Items, pods.Items)
	sm.addPodsToHierarchy(pods.Items)
	sm.sortHierarchy()

	return nil
}

func (sm *StateManager) loadAllNamespaces(ctx context.Context, client client.Client) error {
	var namespaces corev1.NamespaceList
	if err := client.List(ctx, &namespaces); err != nil {
		return fmt.Errorf("failed to list namespaces: %w", err)
	}

	for _, ns := range namespaces.Items {
		nsNode := namespaceToHierarchyNode(&ns)
		sm.hierarchy = append(sm.hierarchy, nsNode)
	}
	return nil
}

func (sm *StateManager) addHTTPRoutesToHierarchy(routes []gatewayv1beta1.HTTPRoute, services []corev1.Service, pods []corev1.Pod) {
	for _, route := range routes {
		if !shouldIncludeHTTPRoute(&route) {
			continue
		}

		namespaceIndex := sm.findNamespaceIndex(route.Namespace)
		if namespaceIndex == -1 {
			continue
		}

		routeNode := httpRouteToHierarchyNode(&route)
		routeNode.Relatives = findServicesForHTTPRoute(&route, services, pods)
		sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, routeNode)
	}
}

func (sm *StateManager) addServicesToHierarchy(services []corev1.Service, pods []corev1.Pod) {
	for _, service := range services {
		if !shouldIncludeService(&service) {
			continue
		}

		namespaceIndex := sm.findNamespaceIndex(service.Namespace)
		if namespaceIndex == -1 {
			continue
		}

		if sm.serviceAlreadyInHTTPRoute(service.Name, sm.hierarchy[namespaceIndex]) {
			continue
		}

		serviceNode := createServiceNodeWithPods(&service, pods)
		sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, serviceNode)
	}
}

func (sm *StateManager) addPodsToHierarchy(pods []corev1.Pod) {
	for _, pod := range pods {
		if !shouldIncludePod(&pod) {
			continue
		}

		namespaceIndex := sm.findNamespaceIndex(pod.Namespace)
		if namespaceIndex == -1 {
			continue
		}

		if podMatchesAnyService(&pod, sm.hierarchy[namespaceIndex]) {
			continue
		}

		podNode := podToHierarchyNode(&pod)
		sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, podNode)
	}
}

func (sm *StateManager) sortHierarchy() {
	slices.SortFunc(sm.hierarchy, func(a, b types.HierarchyNode) int {
		if a.Name < b.Name {
			return -1
		}
		if a.Name > b.Name {
			return 1
		}
		return 0
	})
}

func (sm *StateManager) findNamespaceIndex(namespaceName string) int {
	for i, node := range sm.hierarchy {
		if node.Kind == types.ResourceKindNamespace && node.Name == namespaceName {
			return i
		}
	}
	return -1
}

func (sm *StateManager) serviceAlreadyInHTTPRoute(serviceName string, namespaceNode types.HierarchyNode) bool {
	for _, relative := range namespaceNode.Relatives {
		if relative.Kind != types.ResourceKindHTTPRoute {
			continue
		}
		for _, serviceNode := range relative.Relatives {
			if serviceNode.Kind == types.ResourceKindService && serviceNode.Name == serviceName {
				return true
			}
		}
	}
	return false
}

// addPodToMatchingServiceInNamespace adds the pod to any matching service in the namespace
// returns true if the pod was added to a service, false if it should be added to namespace
func (sm *StateManager) addPodToMatchingServiceInNamespace(namespaceNode *types.HierarchyNode, pod *corev1.Pod, podNode types.HierarchyNode) bool {
	return sm.addPodToMatchingServiceInNode(namespaceNode, pod, podNode)
}

// addPodToMatchingServiceInNode recursively searches for services that match the pod
func (sm *StateManager) addPodToMatchingServiceInNode(node *types.HierarchyNode, pod *corev1.Pod, podNode types.HierarchyNode) bool {
	if node.Kind == types.ResourceKindService {
		serviceNamespace := ""
		if node.Namespace != nil {
			serviceNamespace = *node.Namespace
		}

		if serviceNamespace == pod.Namespace && labelsMatch(node.Selectors, pod.Labels) {
			node.Relatives = slices.DeleteFunc(node.Relatives, func(child types.HierarchyNode) bool {
				return child.Kind == types.ResourceKindPod && child.Name == pod.Name
			})

			node.Relatives = append(node.Relatives, podNode)
			return true
		}
	}

	for i := range node.Relatives {
		if sm.addPodToMatchingServiceInNode(&node.Relatives[i], pod, podNode) {
			return true
		}
	}

	return false
}
