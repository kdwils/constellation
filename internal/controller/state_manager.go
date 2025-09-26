package controller

import (
	"context"
	"fmt"
	"slices"
	"sync"

	"github.com/gorilla/websocket"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/log"
	gatewayv1beta1 "sigs.k8s.io/gateway-api/apis/v1beta1"

	"github.com/kdwils/constellation/internal/types"
)

type StateUpdateEvent struct {
	Type      StateUpdateType
	Namespace corev1.Namespace
	Service   corev1.Service
	Pod       corev1.Pod
	HTTPRoute gatewayv1beta1.HTTPRoute
	Pods      []corev1.Pod
	Services  []corev1.Service
	Name      string
	Ns        string
}

type StateUpdateType int

const (
	NamespaceUpdate StateUpdateType = iota
	NamespaceDelete
	ServiceUpdate
	ServiceDelete
	PodUpdate
	PodDelete
	HTTPRouteUpdate
	HTTPRouteDelete
)

// StateManager maintains the hierarchy state and serves it via HTTP
type StateManager struct {
	hierarchy   []types.HierarchyNode
	mu          sync.RWMutex
	subscribers map[chan []types.HierarchyNode]bool
	subMu       sync.RWMutex
	updateChan  chan bool
	eventChan   chan StateUpdateEvent
}

// NewStateManager creates a new state manager
func NewStateManager(updateChan chan bool) *StateManager {
	return &StateManager{
		hierarchy:   make([]types.HierarchyNode, 0),
		subscribers: make(map[chan []types.HierarchyNode]bool),
		updateChan:  updateChan,
		eventChan:   make(chan StateUpdateEvent, 100),
	}
}

// BaseReconciler provides common functionality for all resource reconcilers
type BaseReconciler struct {
	client.Client
	Scheme       *runtime.Scheme
	stateManager *StateManager
}

// newBaseReconciler creates a new BaseReconciler with common initialization
func newBaseReconciler(mgr ctrl.Manager, stateManager *StateManager) BaseReconciler {
	return BaseReconciler{
		Client:       mgr.GetClient(),
		Scheme:       mgr.GetScheme(),
		stateManager: stateManager,
	}
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

// PushUpdate safely writes hierarchy to WebSocket connection while holding mutex
func (sm *StateManager) Start(ctx context.Context) {
	go sm.processUpdates(ctx)
}

func (sm *StateManager) processUpdates(ctx context.Context) {
	for {
		select {
		case event := <-sm.eventChan:
			sm.handleUpdate(event)
		case <-ctx.Done():
			return
		}
	}
}

func (sm *StateManager) handleUpdate(event StateUpdateEvent) {
	sm.mu.Lock()
	defer sm.mu.Unlock()

	switch event.Type {
	case NamespaceUpdate:
		sm.processNamespaceUpdate(event.Namespace)
	case NamespaceDelete:
		sm.processNamespaceDelete(event.Name)
	case ServiceUpdate:
		sm.processServiceUpdate(event.Service, event.Pods)
	case ServiceDelete:
		for i := range sm.hierarchy {
			removeNodeByKind(&sm.hierarchy[i], types.ResourceKindService, event.Name, event.Ns)
		}
	case PodUpdate:
		sm.processPodUpdate(event.Pod)
	case PodDelete:
		for i := range sm.hierarchy {
			removeNodeByKind(&sm.hierarchy[i], types.ResourceKindPod, event.Name, event.Ns)
		}
	case HTTPRouteUpdate:
		sm.processHTTPRouteUpdate(event.HTTPRoute, event.Services, event.Pods)
	case HTTPRouteDelete:
		for i := range sm.hierarchy {
			removeNodeByKind(&sm.hierarchy[i], types.ResourceKindHTTPRoute, event.Name, event.Ns)
		}
	}

	sm.notifySubscribers()
}

func (sm *StateManager) notifySubscribers() {
	sm.subMu.RLock()
	defer sm.subMu.RUnlock()

	for subscriber := range sm.subscribers {
		select {
		case subscriber <- nil:
		default:
		}
	}

	sm.updateChan <- true
}

func (sm *StateManager) processNamespaceUpdate(namespace corev1.Namespace) {
	_, found := sm.findNamespaceIndex(namespace.Name)
	if !found {
		newNode := namespaceToHierarchyNode(namespace)
		sm.hierarchy = append(sm.hierarchy, newNode)
	}

	slices.SortFunc(sm.hierarchy, compareNodesByName)
}

func (sm *StateManager) processNamespaceDelete(name string) {
	sm.hierarchy = slices.DeleteFunc(sm.hierarchy, func(node types.HierarchyNode) bool {
		return node.Kind == types.ResourceKindNamespace && node.Name == name
	})
}

func (sm *StateManager) processServiceUpdate(service corev1.Service, pods []corev1.Pod) {
	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindService, service.Name, service.Namespace)
	}

	namespaceIndex, found := sm.findNamespaceIndex(service.Namespace)
	if !found {
		return
	}

	httpRouteIndex := findHTTPRouteForService(sm.hierarchy[namespaceIndex], service.Name)
	serviceNode := createServiceNodeWithPods(service, pods)

	if httpRouteIndex != -1 {
		sm.hierarchy[namespaceIndex].Relatives[httpRouteIndex].Relatives = append(
			sm.hierarchy[namespaceIndex].Relatives[httpRouteIndex].Relatives, serviceNode)
		return
	}

	sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, serviceNode)
}

func (sm *StateManager) processPodUpdate(pod corev1.Pod) {
	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindPod, pod.Name, pod.Namespace)
	}

	if !shouldIncludePod(pod) {
		return
	}

	namespaceIndex, found := sm.findNamespaceIndex(pod.Namespace)
	if !found {
		return
	}

	podNode := podToHierarchyNode(pod)

	foundMatchingService := false
	for i := range sm.hierarchy[namespaceIndex].Relatives {
		if sm.hierarchy[namespaceIndex].Relatives[i].Kind != types.ResourceKindService {
			continue
		}
		serviceNamespace := ""
		if sm.hierarchy[namespaceIndex].Relatives[i].Namespace != nil {
			serviceNamespace = *sm.hierarchy[namespaceIndex].Relatives[i].Namespace
		}
		if serviceNamespace != pod.Namespace {
			continue
		}
		if !labelsMatch(sm.hierarchy[namespaceIndex].Relatives[i].Selectors, pod.Labels) {
			continue
		}

		sm.hierarchy[namespaceIndex].Relatives[i].Relatives = slices.DeleteFunc(
			sm.hierarchy[namespaceIndex].Relatives[i].Relatives,
			func(child types.HierarchyNode) bool {
				return child.Kind == types.ResourceKindPod && child.Name == pod.Name
			})
		sm.hierarchy[namespaceIndex].Relatives[i].Relatives = append(sm.hierarchy[namespaceIndex].Relatives[i].Relatives, podNode)
		foundMatchingService = true
		break
	}

	if !foundMatchingService {
		sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, podNode)
	}
}

func (sm *StateManager) processHTTPRouteUpdate(route gatewayv1beta1.HTTPRoute, services []corev1.Service, pods []corev1.Pod) {
	for i := range sm.hierarchy {
		removeNodeByKind(&sm.hierarchy[i], types.ResourceKindHTTPRoute, route.Name, route.Namespace)
	}

	namespaceIndex, found := sm.findNamespaceIndex(route.Namespace)
	if !found {
		return
	}

	routeNode := httpRouteToHierarchyNode(route)
	routeNode.Relatives = findServicesForHTTPRoute(route, services, pods)
	sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, routeNode)
}

func (sm *StateManager) UpdateNamespace(ctx context.Context, ns corev1.Namespace) {
	logger := log.FromContext(ctx)
	logger.V(1).Info("Updating namespace", "namespace", ns.Name)
	sm.eventChan <- StateUpdateEvent{Type: NamespaceUpdate, Namespace: ns}
}

func (sm *StateManager) DeleteNamespace(ctx context.Context, name string) {
	logger := log.FromContext(ctx)
	logger.V(1).Info("Deleting namespace", "namespace", name)
	sm.eventChan <- StateUpdateEvent{Type: NamespaceDelete, Name: name}
}

func (sm *StateManager) UpdateService(ctx context.Context, svc corev1.Service, pods []corev1.Pod) {
	logger := log.FromContext(ctx)
	logger.V(1).Info("Updating service", "service", svc.Name, "namespace", svc.Namespace, "podCount", len(pods))
	sm.eventChan <- StateUpdateEvent{Type: ServiceUpdate, Service: svc, Pods: pods}
}

func (sm *StateManager) DeleteService(ctx context.Context, name, namespace string) {
	logger := log.FromContext(ctx)
	logger.V(1).Info("Deleting service", "service", name, "namespace", namespace)
	sm.eventChan <- StateUpdateEvent{Type: ServiceDelete, Name: name, Ns: namespace}
}

func (sm *StateManager) UpdatePod(ctx context.Context, pod corev1.Pod) {
	logger := log.FromContext(ctx)
	logger.V(1).Info("Updating pod", "pod", pod.Name, "namespace", pod.Namespace, "phase", pod.Status.Phase)
	sm.eventChan <- StateUpdateEvent{Type: PodUpdate, Pod: pod}
}

func (sm *StateManager) DeletePod(ctx context.Context, name, namespace string) {
	logger := log.FromContext(ctx)
	logger.V(1).Info("Deleting pod", "pod", name, "namespace", namespace)
	sm.eventChan <- StateUpdateEvent{Type: PodDelete, Name: name, Ns: namespace}
}

func (sm *StateManager) UpdateHTTPRoute(ctx context.Context, route gatewayv1beta1.HTTPRoute, services []corev1.Service, pods []corev1.Pod) {
	logger := log.FromContext(ctx)
	logger.V(1).Info("Updating httproute", "httproute", route.Name, "namespace", route.Namespace, "serviceCount", len(services), "podCount", len(pods))
	sm.eventChan <- StateUpdateEvent{Type: HTTPRouteUpdate, HTTPRoute: route, Services: services, Pods: pods}
}

func (sm *StateManager) DeleteHTTPRoute(ctx context.Context, name, namespace string) {
	logger := log.FromContext(ctx)
	logger.V(1).Info("Deleting httproute", "httproute", name, "namespace", namespace)
	sm.eventChan <- StateUpdateEvent{Type: HTTPRouteDelete, Name: name, Ns: namespace}
}

func (sm *StateManager) PushUpdate(conn *websocket.Conn) error {
	sm.mu.RLock()
	defer sm.mu.RUnlock()
	return conn.WriteJSON(sm.hierarchy)
}

func (sm *StateManager) Subscribe() chan []types.HierarchyNode {
	sm.subMu.Lock()
	defer sm.subMu.Unlock()

	ch := make(chan []types.HierarchyNode, 10)
	sm.subscribers[ch] = true
	return ch
}

func (sm *StateManager) Unsubscribe(ch chan []types.HierarchyNode) {
	sm.subMu.Lock()
	defer sm.subMu.Unlock()

	delete(sm.subscribers, ch)
	close(ch)
}

// Helper functions to check if a resource should be included
func shouldIncludePod(pod corev1.Pod) bool {
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
func namespaceToHierarchyNode(ns corev1.Namespace) types.HierarchyNode {
	return types.HierarchyNode{
		Kind:      types.ResourceKindNamespace,
		Name:      ns.Name,
		Namespace: nil,
		Relatives: []types.HierarchyNode{},
		Labels:    ns.Labels,
	}
}

func toHierarchyNode(kind types.ResourceKind, name, namespace string, metadata types.ResourceMetadata) types.HierarchyNode {
	return types.HierarchyNode{
		Kind:            kind,
		Name:            name,
		Namespace:       stringToPtr(namespace),
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

func stringToPtr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func serviceToHierarchyNode(svc corev1.Service) types.HierarchyNode {
	metadata := extractServiceMetadata(svc)
	return toHierarchyNode(types.ResourceKindService, svc.Name, svc.Namespace, metadata)
}

func podToHierarchyNode(pod corev1.Pod) types.HierarchyNode {
	metadata := extractPodMetadata(pod)
	return toHierarchyNode(types.ResourceKindPod, pod.Name, pod.Namespace, metadata)
}

func httpRouteToHierarchyNode(route gatewayv1beta1.HTTPRoute) types.HierarchyNode {
	metadata := extractHTTPRouteMetadata(route)
	return toHierarchyNode(types.ResourceKindHTTPRoute, route.Name, route.Namespace, metadata)
}

func getAnnotationValue(annotations map[string]string, key string) string {
	if annotations == nil {
		return ""
	}
	return annotations[key]
}

func extractGroupFromAnnotations(annotations map[string]string) string {
	return getAnnotationValue(annotations, "constellation.kyledev.co/group")
}

func extractDisplayNameFromAnnotations(annotations map[string]string) string {
	return getAnnotationValue(annotations, "constellation.kyledev.co/display-name")
}

func shouldIgnoreResource(annotations map[string]string) bool {
	return getAnnotationValue(annotations, "constellation.kyledev.co/ignore") == "true"
}

func extractServiceMetadata(svc corev1.Service) types.ResourceMetadata {
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

func extractPodMetadata(pod corev1.Pod) types.ResourceMetadata {
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

func extractHTTPRouteMetadata(route gatewayv1beta1.HTTPRoute) types.ResourceMetadata {
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
	return slices.Contains(httpRoute.BackendRefs, serviceName)
}

// findMatchingPods returns all pods that match the service selector
func findMatchingPods(service *corev1.Service, pods []corev1.Pod) []types.HierarchyNode {
	var matchingPods []types.HierarchyNode
	for _, pod := range pods {
		if pod.Namespace != service.Namespace {
			continue
		}
		if !shouldIncludePod(pod) {
			continue
		}
		if !labelsMatch(service.Spec.Selector, pod.Labels) {
			continue
		}
		matchingPods = append(matchingPods, podToHierarchyNode(pod))
	}
	return matchingPods
}

// createServiceNodeWithPods creates a service node with matching pods already included
func createServiceNodeWithPods(service corev1.Service, pods []corev1.Pod) types.HierarchyNode {
	serviceNode := serviceToHierarchyNode(service)
	serviceNode.Relatives = findMatchingPods(&service, pods)
	return serviceNode
}

// findServiceByName returns the service with matching name and namespace
func findServiceByName(services []corev1.Service, name, namespace string) (corev1.Service, bool) {
	for _, service := range services {
		if service.Name != name {
			continue
		}
		if service.Namespace != namespace {
			continue
		}
		return service, true
	}
	return corev1.Service{}, false
}

// findServicesForHTTPRoute returns all services referenced by the HTTPRoute
func findServicesForHTTPRoute(route gatewayv1beta1.HTTPRoute, services []corev1.Service, pods []corev1.Pod) []types.HierarchyNode {
	var serviceNodes []types.HierarchyNode
	for _, rule := range route.Spec.Rules {
		for _, backendRef := range rule.BackendRefs {
			serviceName := string(backendRef.Name)
			service, found := findServiceByName(services, serviceName, route.Namespace)
			if !found {
				continue
			}
			serviceNode := createServiceNodeWithPods(service, pods)
			serviceNodes = append(serviceNodes, serviceNode)
		}
	}
	return serviceNodes
}

// podMatchesAnyService checks if a pod matches any service selector in the namespace
func podMatchesAnyService(pod corev1.Pod, node types.HierarchyNode) bool {
	if node.Kind != types.ResourceKindService {
		return slices.ContainsFunc(node.Relatives, func(relative types.HierarchyNode) bool {
			return podMatchesAnyService(pod, relative)
		})
	}

	if node.Namespace == nil {
		return slices.ContainsFunc(node.Relatives, func(relative types.HierarchyNode) bool {
			return podMatchesAnyService(pod, relative)
		})
	}

	if *node.Namespace != pod.Namespace {
		return slices.ContainsFunc(node.Relatives, func(relative types.HierarchyNode) bool {
			return podMatchesAnyService(pod, relative)
		})
	}

	if !labelsMatch(node.Selectors, pod.Labels) {
		return slices.ContainsFunc(node.Relatives, func(relative types.HierarchyNode) bool {
			return podMatchesAnyService(pod, relative)
		})
	}

	return true
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

	for _, route := range httpRoutes.Items {
		namespaceIndex, found := sm.findNamespaceIndex(route.Namespace)
		if !found {
			continue
		}
		routeNode := httpRouteToHierarchyNode(route)
		routeNode.Relatives = findServicesForHTTPRoute(route, services.Items, pods.Items)
		sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, routeNode)
	}

	for _, service := range services.Items {
		namespaceIndex, found := sm.findNamespaceIndex(service.Namespace)
		if !found {
			continue
		}
		if sm.serviceAlreadyInHTTPRoute(service.Name, sm.hierarchy[namespaceIndex]) {
			continue
		}
		serviceNode := createServiceNodeWithPods(service, pods.Items)
		sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, serviceNode)
	}

	for _, pod := range pods.Items {
		if !shouldIncludePod(pod) {
			continue
		}
		namespaceIndex, found := sm.findNamespaceIndex(pod.Namespace)
		if !found {
			continue
		}
		if podMatchesAnyService(pod, sm.hierarchy[namespaceIndex]) {
			continue
		}
		podNode := podToHierarchyNode(pod)
		sm.hierarchy[namespaceIndex].Relatives = append(sm.hierarchy[namespaceIndex].Relatives, podNode)
	}

	slices.SortFunc(sm.hierarchy, compareNodesByName)

	return nil
}

func (sm *StateManager) loadAllNamespaces(ctx context.Context, client client.Client) error {
	var namespaces corev1.NamespaceList
	if err := client.List(ctx, &namespaces); err != nil {
		return fmt.Errorf("failed to list namespaces: %w", err)
	}

	for _, ns := range namespaces.Items {
		nsNode := namespaceToHierarchyNode(ns)
		sm.hierarchy = append(sm.hierarchy, nsNode)
	}
	return nil
}

func compareNodesByName(a, b types.HierarchyNode) int {
	if a.Name < b.Name {
		return -1
	}
	if a.Name > b.Name {
		return 1
	}
	return 0
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

func (sm *StateManager) findNamespaceIndex(namespaceName string) (int, bool) {
	for i, node := range sm.hierarchy {
		if node.Kind != types.ResourceKindNamespace {
			continue
		}
		if node.Name != namespaceName {
			continue
		}
		return i, true
	}
	return -1, false
}
