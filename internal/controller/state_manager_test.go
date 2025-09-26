package controller

import (
	"testing"
	"time"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/util/intstr"
	gatewayv1beta1 "sigs.k8s.io/gateway-api/apis/v1beta1"

	"github.com/kdwils/constellation/internal/types"
)

func TestStateManager_findNamespaceIndex(t *testing.T) {
	tests := []struct {
		name          string
		updateChan    chan bool
		namespaceName string
		want          int
		want2         bool
	}{
		{
			name:          "find existing namespace",
			updateChan:    make(chan bool, 1),
			namespaceName: "namespace-b",
			want:          1,
			want2:         true,
		},
		{
			name:          "namespace not found",
			updateChan:    make(chan bool, 1),
			namespaceName: "missing-namespace",
			want:          -1,
			want2:         false,
		},
		{
			name:          "first namespace",
			updateChan:    make(chan bool, 1),
			namespaceName: "namespace-a",
			want:          0,
			want2:         true,
		},
		{
			name:          "empty name",
			updateChan:    make(chan bool, 1),
			namespaceName: "",
			want:          -1,
			want2:         false,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			sm := NewStateManager(tt.updateChan)
			// Set up test hierarchy
			sm.hierarchy = []types.HierarchyNode{
				{
					Name: "namespace-a",
					Kind: types.ResourceKindNamespace,
				},
				{
					Name: "namespace-b",
					Kind: types.ResourceKindNamespace,
				},
			}
			got, got2 := sm.findNamespaceIndex(tt.namespaceName)
			if got != tt.want {
				t.Errorf("findNamespaceIndex() got = %v, want %v", got, tt.want)
			}
			if got2 != tt.want2 {
				t.Errorf("findNamespaceIndex() got2 = %v, want %v", got2, tt.want2)
			}
		})
	}
}

func TestStateManager_serviceAlreadyInHTTPRoute(t *testing.T) {
	tests := []struct {
		name          string
		updateChan    chan bool
		serviceName   string
		namespaceNode types.HierarchyNode
		want          bool
	}{
		{
			name:        "service exists in HTTPRoute",
			updateChan:  make(chan bool, 1),
			serviceName: "test-service",
			namespaceNode: types.HierarchyNode{
				Name: "test-namespace",
				Kind: types.ResourceKindNamespace,
				Relatives: []types.HierarchyNode{
					{
						Name: "test-route",
						Kind: types.ResourceKindHTTPRoute,
						Relatives: []types.HierarchyNode{
							{
								Name: "test-service",
								Kind: types.ResourceKindService,
							},
						},
					},
				},
			},
			want: true,
		},
		{
			name:        "service does not exist in HTTPRoute",
			updateChan:  make(chan bool, 1),
			serviceName: "missing-service",
			namespaceNode: types.HierarchyNode{
				Name: "test-namespace",
				Kind: types.ResourceKindNamespace,
				Relatives: []types.HierarchyNode{
					{
						Name: "test-route",
						Kind: types.ResourceKindHTTPRoute,
						Relatives: []types.HierarchyNode{
							{
								Name: "test-service",
								Kind: types.ResourceKindService,
							},
						},
					},
				},
			},
			want: false,
		},
		{
			name:        "no HTTPRoute in namespace",
			updateChan:  make(chan bool, 1),
			serviceName: "test-service",
			namespaceNode: types.HierarchyNode{
				Name: "test-namespace",
				Kind: types.ResourceKindNamespace,
				Relatives: []types.HierarchyNode{
					{
						Name: "test-service",
						Kind: types.ResourceKindService,
					},
				},
			},
			want: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			sm := NewStateManager(tt.updateChan)
			got := sm.serviceAlreadyInHTTPRoute(tt.serviceName, tt.namespaceNode)
			if got != tt.want {
				t.Errorf("serviceAlreadyInHTTPRoute() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_shouldIncludePod(t *testing.T) {
	now := time.Now()
	deletionTime := &metav1.Time{Time: now}

	tests := []struct {
		name string
		pod  corev1.Pod
		want bool
	}{
		{
			name: "running pod should be included",
			pod: corev1.Pod{
				Status: corev1.PodStatus{
					Phase: corev1.PodRunning,
				},
			},
			want: true,
		},
		{
			name: "pending pod should be included",
			pod: corev1.Pod{
				Status: corev1.PodStatus{
					Phase: corev1.PodPending,
				},
			},
			want: true,
		},
		{
			name: "failed pod should not be included",
			pod: corev1.Pod{
				Status: corev1.PodStatus{
					Phase: corev1.PodFailed,
				},
			},
			want: false,
		},
		{
			name: "succeeded pod should not be included",
			pod: corev1.Pod{
				Status: corev1.PodStatus{
					Phase: corev1.PodSucceeded,
				},
			},
			want: false,
		},
		{
			name: "pod with deletion timestamp should not be included",
			pod: corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					DeletionTimestamp: deletionTime,
				},
				Status: corev1.PodStatus{
					Phase: corev1.PodRunning,
				},
			},
			want: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := shouldIncludePod(tt.pod)
			if got != tt.want {
				t.Errorf("shouldIncludePod() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_labelsMatch(t *testing.T) {
	tests := []struct {
		name      string
		selectors map[string]string
		labels    map[string]string
		want      bool
	}{
		{
			name:      "exact match",
			selectors: map[string]string{"app": "test", "version": "1.0"},
			labels:    map[string]string{"app": "test", "version": "1.0"},
			want:      true,
		},
		{
			name:      "labels have extra keys",
			selectors: map[string]string{"app": "test"},
			labels:    map[string]string{"app": "test", "version": "1.0", "env": "prod"},
			want:      true,
		},
		{
			name:      "selector value mismatch",
			selectors: map[string]string{"app": "test", "version": "1.0"},
			labels:    map[string]string{"app": "test", "version": "2.0"},
			want:      false,
		},
		{
			name:      "missing selector key in labels",
			selectors: map[string]string{"app": "test", "version": "1.0"},
			labels:    map[string]string{"app": "test"},
			want:      false,
		},
		{
			name:      "nil selectors",
			selectors: nil,
			labels:    map[string]string{"app": "test"},
			want:      false,
		},
		{
			name:      "nil labels",
			selectors: map[string]string{"app": "test"},
			labels:    nil,
			want:      false,
		},
		{
			name:      "empty selectors",
			selectors: map[string]string{},
			labels:    map[string]string{"app": "test"},
			want:      true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := labelsMatch(tt.selectors, tt.labels)
			if got != tt.want {
				t.Errorf("labelsMatch() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_removeNodeByKind(t *testing.T) {
	namespace := "test-ns"

	tests := []struct {
		name        string
		node        types.HierarchyNode
		kind        types.ResourceKind
		nodeName    string
		nodeNs      string
		wantRemoved bool
	}{
		{
			name: "remove existing pod from service",
			node: types.HierarchyNode{
				Name: "test-service",
				Kind: types.ResourceKindService,
				Relatives: []types.HierarchyNode{
					{
						Name:      "pod-1",
						Kind:      types.ResourceKindPod,
						Namespace: &namespace,
					},
					{
						Name:      "pod-2",
						Kind:      types.ResourceKindPod,
						Namespace: &namespace,
					},
				},
			},
			kind:        types.ResourceKindPod,
			nodeName:    "pod-1",
			nodeNs:      "test-ns",
			wantRemoved: true,
		},
		{
			name: "try to remove non-existent pod",
			node: types.HierarchyNode{
				Name: "test-service",
				Kind: types.ResourceKindService,
				Relatives: []types.HierarchyNode{
					{
						Name:      "pod-1",
						Kind:      types.ResourceKindPod,
						Namespace: &namespace,
					},
				},
			},
			kind:        types.ResourceKindPod,
			nodeName:    "missing-pod",
			nodeNs:      "test-ns",
			wantRemoved: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			initialCount := len(tt.node.Relatives)
			removeNodeByKind(&tt.node, tt.kind, tt.nodeName, tt.nodeNs)
			finalCount := len(tt.node.Relatives)

			if tt.wantRemoved && finalCount != initialCount-1 {
				t.Errorf("removeNodeByKind() should have removed node, initial=%d, final=%d", initialCount, finalCount)
			}
			if !tt.wantRemoved && finalCount != initialCount {
				t.Errorf("removeNodeByKind() should not have removed node, initial=%d, final=%d", initialCount, finalCount)
			}
		})
	}
}

// Utility function tests
func Test_compareNodesByName(t *testing.T) {
	tests := []struct {
		name string
		a    types.HierarchyNode
		b    types.HierarchyNode
		want int
	}{
		{
			name: "a comes before b",
			a:    types.HierarchyNode{Name: "apple"},
			b:    types.HierarchyNode{Name: "banana"},
			want: -1,
		},
		{
			name: "a comes after b",
			a:    types.HierarchyNode{Name: "zebra"},
			b:    types.HierarchyNode{Name: "apple"},
			want: 1,
		},
		{
			name: "a equals b",
			a:    types.HierarchyNode{Name: "same"},
			b:    types.HierarchyNode{Name: "same"},
			want: 0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := compareNodesByName(tt.a, tt.b)
			if got != tt.want {
				t.Errorf("compareNodesByName() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_stringToPtr(t *testing.T) {
	tests := []struct {
		name string
		s    string
		want *string
	}{
		{
			name: "non-empty string",
			s:    "test",
			want: func() *string { s := "test"; return &s }(),
		},
		{
			name: "empty string",
			s:    "",
			want: nil,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := stringToPtr(tt.s)
			if tt.want == nil && got != nil {
				t.Errorf("stringToPtr() = %v, want nil", got)
			} else if tt.want != nil && (got == nil || *got != *tt.want) {
				t.Errorf("stringToPtr() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_getAnnotationValue(t *testing.T) {
	tests := []struct {
		name        string
		annotations map[string]string
		key         string
		want        string
	}{
		{
			name:        "existing key",
			annotations: map[string]string{"app": "test", "version": "1.0"},
			key:         "app",
			want:        "test",
		},
		{
			name:        "non-existing key",
			annotations: map[string]string{"app": "test"},
			key:         "missing",
			want:        "",
		},
		{
			name:        "nil annotations",
			annotations: nil,
			key:         "app",
			want:        "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := getAnnotationValue(tt.annotations, tt.key)
			if got != tt.want {
				t.Errorf("getAnnotationValue() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_extractGroupFromAnnotations(t *testing.T) {
	tests := []struct {
		name        string
		annotations map[string]string
		want        string
	}{
		{
			name:        "has group annotation",
			annotations: map[string]string{"constellation.kyledev.co/group": "my-group"},
			want:        "my-group",
		},
		{
			name:        "no group annotation",
			annotations: map[string]string{"other": "value"},
			want:        "",
		},
		{
			name:        "nil annotations",
			annotations: nil,
			want:        "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := extractGroupFromAnnotations(tt.annotations)
			if got != tt.want {
				t.Errorf("extractGroupFromAnnotations() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_shouldIgnoreResource(t *testing.T) {
	tests := []struct {
		name        string
		annotations map[string]string
		want        bool
	}{
		{
			name:        "ignore true",
			annotations: map[string]string{"constellation.kyledev.co/ignore": "true"},
			want:        true,
		},
		{
			name:        "ignore false",
			annotations: map[string]string{"constellation.kyledev.co/ignore": "false"},
			want:        false,
		},
		{
			name:        "no ignore annotation",
			annotations: map[string]string{"other": "value"},
			want:        false,
		},
		{
			name:        "nil annotations",
			annotations: nil,
			want:        false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := shouldIgnoreResource(tt.annotations)
			if got != tt.want {
				t.Errorf("shouldIgnoreResource() = %v, want %v", got, tt.want)
			}
		})
	}
}

// Resource conversion function tests
func Test_namespaceToHierarchyNode(t *testing.T) {
	tests := []struct {
		name      string
		namespace corev1.Namespace
		want      types.HierarchyNode
	}{
		{
			name: "basic namespace",
			namespace: corev1.Namespace{
				ObjectMeta: metav1.ObjectMeta{
					Name: "test-ns",
					Labels: map[string]string{
						"env": "test",
					},
				},
			},
			want: types.HierarchyNode{
				Kind:      types.ResourceKindNamespace,
				Name:      "test-ns",
				Namespace: nil,
				Relatives: []types.HierarchyNode{},
				Labels: map[string]string{
					"env": "test",
				},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := namespaceToHierarchyNode(tt.namespace)
			if got.Kind != tt.want.Kind || got.Name != tt.want.Name {
				t.Errorf("namespaceToHierarchyNode() = %v, want %v", got, tt.want)
			}
			if len(got.Labels) != len(tt.want.Labels) {
				t.Errorf("namespaceToHierarchyNode() labels = %v, want %v", got.Labels, tt.want.Labels)
			}
		})
	}
}

func Test_serviceToHierarchyNode(t *testing.T) {
	tests := []struct {
		name    string
		service corev1.Service
		want    types.HierarchyNode
	}{
		{
			name: "basic service",
			service: corev1.Service{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "test-service",
					Namespace: "test-ns",
					Labels: map[string]string{
						"app": "test",
					},
				},
				Spec: corev1.ServiceSpec{
					Selector: map[string]string{
						"app": "test",
					},
					Type: corev1.ServiceTypeClusterIP,
					Ports: []corev1.ServicePort{
						{
							Port:       80,
							TargetPort: intstr.FromInt(8080),
						},
					},
				},
			},
			want: types.HierarchyNode{
				Kind: types.ResourceKindService,
				Name: "test-service",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := serviceToHierarchyNode(tt.service)
			if got.Kind != tt.want.Kind || got.Name != tt.want.Name {
				t.Errorf("serviceToHierarchyNode() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_podToHierarchyNode(t *testing.T) {
	tests := []struct {
		name string
		pod  corev1.Pod
		want types.HierarchyNode
	}{
		{
			name: "basic pod",
			pod: corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "test-pod",
					Namespace: "test-ns",
					Labels: map[string]string{
						"app": "test",
					},
				},
				Status: corev1.PodStatus{
					Phase: corev1.PodRunning,
				},
			},
			want: types.HierarchyNode{
				Kind: types.ResourceKindPod,
				Name: "test-pod",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := podToHierarchyNode(tt.pod)
			if got.Kind != tt.want.Kind || got.Name != tt.want.Name {
				t.Errorf("podToHierarchyNode() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_httpRouteToHierarchyNode(t *testing.T) {
	tests := []struct {
		name  string
		route gatewayv1beta1.HTTPRoute
		want  types.HierarchyNode
	}{
		{
			name: "basic HTTPRoute",
			route: gatewayv1beta1.HTTPRoute{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "test-route",
					Namespace: "test-ns",
				},
				Spec: gatewayv1beta1.HTTPRouteSpec{
					Hostnames: []gatewayv1beta1.Hostname{
						"example.com",
					},
				},
			},
			want: types.HierarchyNode{
				Kind: types.ResourceKindHTTPRoute,
				Name: "test-route",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := httpRouteToHierarchyNode(tt.route)
			if got.Kind != tt.want.Kind || got.Name != tt.want.Name {
				t.Errorf("httpRouteToHierarchyNode() = %v, want %v", got, tt.want)
			}
		})
	}
}

// Search/matching function tests
func Test_findHTTPRouteForService(t *testing.T) {
	tests := []struct {
		name          string
		namespaceNode types.HierarchyNode
		serviceName   string
		want          int
	}{
		{
			name: "service found in HTTPRoute",
			namespaceNode: types.HierarchyNode{
				Name: "test-namespace",
				Kind: types.ResourceKindNamespace,
				Relatives: []types.HierarchyNode{
					{
						Name:        "test-route",
						Kind:        types.ResourceKindHTTPRoute,
						BackendRefs: []string{"target-service", "other-service"},
					},
				},
			},
			serviceName: "target-service",
			want:        0,
		},
		{
			name: "service not found",
			namespaceNode: types.HierarchyNode{
				Name: "test-namespace",
				Kind: types.ResourceKindNamespace,
				Relatives: []types.HierarchyNode{
					{
						Name:        "test-route",
						Kind:        types.ResourceKindHTTPRoute,
						BackendRefs: []string{"other-service"},
					},
				},
			},
			serviceName: "missing-service",
			want:        -1,
		},
		{
			name: "no HTTPRoute in namespace",
			namespaceNode: types.HierarchyNode{
				Name: "test-namespace",
				Kind: types.ResourceKindNamespace,
				Relatives: []types.HierarchyNode{
					{
						Name: "test-service",
						Kind: types.ResourceKindService,
					},
				},
			},
			serviceName: "test-service",
			want:        -1,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := findHTTPRouteForService(tt.namespaceNode, tt.serviceName)
			if got != tt.want {
				t.Errorf("findHTTPRouteForService() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_serviceReferencedByHTTPRoute(t *testing.T) {
	tests := []struct {
		name        string
		httpRoute   types.HierarchyNode
		serviceName string
		want        bool
	}{
		{
			name: "service is referenced",
			httpRoute: types.HierarchyNode{
				Name:        "test-route",
				Kind:        types.ResourceKindHTTPRoute,
				BackendRefs: []string{"service-a", "service-b"},
			},
			serviceName: "service-b",
			want:        true,
		},
		{
			name: "service is not referenced",
			httpRoute: types.HierarchyNode{
				Name:        "test-route",
				Kind:        types.ResourceKindHTTPRoute,
				BackendRefs: []string{"service-a", "service-b"},
			},
			serviceName: "service-c",
			want:        false,
		},
		{
			name: "empty backend refs",
			httpRoute: types.HierarchyNode{
				Name:        "test-route",
				Kind:        types.ResourceKindHTTPRoute,
				BackendRefs: []string{},
			},
			serviceName: "service-a",
			want:        false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := serviceReferencedByHTTPRoute(tt.httpRoute, tt.serviceName)
			if got != tt.want {
				t.Errorf("serviceReferencedByHTTPRoute() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_findServiceByName(t *testing.T) {
	services := []corev1.Service{
		{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "service-a",
				Namespace: "ns1",
			},
		},
		{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "service-b",
				Namespace: "ns1",
			},
		},
		{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "service-a",
				Namespace: "ns2",
			},
		},
	}

	tests := []struct {
		name        string
		services    []corev1.Service
		serviceName string
		namespace   string
		wantFound   bool
		wantName    string
	}{
		{
			name:        "service found",
			services:    services,
			serviceName: "service-a",
			namespace:   "ns1",
			wantFound:   true,
			wantName:    "service-a",
		},
		{
			name:        "service not found - wrong namespace",
			services:    services,
			serviceName: "service-a",
			namespace:   "ns3",
			wantFound:   false,
		},
		{
			name:        "service not found - wrong name",
			services:    services,
			serviceName: "service-c",
			namespace:   "ns1",
			wantFound:   false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, found := findServiceByName(tt.services, tt.serviceName, tt.namespace)
			if found != tt.wantFound {
				t.Errorf("findServiceByName() found = %v, want %v", found, tt.wantFound)
			}
			if found && got.Name != tt.wantName {
				t.Errorf("findServiceByName() name = %v, want %v", got.Name, tt.wantName)
			}
		})
	}
}

func Test_findMatchingPods(t *testing.T) {
	pods := []corev1.Pod{
		{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "pod-a",
				Namespace: "test-ns",
				Labels: map[string]string{
					"app": "test",
				},
			},
			Status: corev1.PodStatus{
				Phase: corev1.PodRunning,
			},
		},
		{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "pod-b",
				Namespace: "test-ns",
				Labels: map[string]string{
					"app": "other",
				},
			},
			Status: corev1.PodStatus{
				Phase: corev1.PodRunning,
			},
		},
		{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "pod-c",
				Namespace: "other-ns",
				Labels: map[string]string{
					"app": "test",
				},
			},
			Status: corev1.PodStatus{
				Phase: corev1.PodRunning,
			},
		},
	}

	tests := []struct {
		name     string
		service  corev1.Service
		pods     []corev1.Pod
		wantLen  int
		wantName string
	}{
		{
			name: "matching pods found",
			service: corev1.Service{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "test-service",
					Namespace: "test-ns",
				},
				Spec: corev1.ServiceSpec{
					Selector: map[string]string{
						"app": "test",
					},
				},
			},
			pods:     pods,
			wantLen:  1,
			wantName: "pod-a",
		},
		{
			name: "no matching pods",
			service: corev1.Service{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "test-service",
					Namespace: "test-ns",
				},
				Spec: corev1.ServiceSpec{
					Selector: map[string]string{
						"app": "missing",
					},
				},
			},
			pods:    pods,
			wantLen: 0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := findMatchingPods(&tt.service, tt.pods)
			if len(got) != tt.wantLen {
				t.Errorf("findMatchingPods() len = %v, want %v", len(got), tt.wantLen)
			}
			if tt.wantLen > 0 && got[0].Name != tt.wantName {
				t.Errorf("findMatchingPods() first pod name = %v, want %v", got[0].Name, tt.wantName)
			}
		})
	}
}

// Node creation function tests
func Test_createServiceNodeWithPods(t *testing.T) {
	service := corev1.Service{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "test-service",
			Namespace: "test-ns",
		},
		Spec: corev1.ServiceSpec{
			Selector: map[string]string{
				"app": "test",
			},
		},
	}

	pods := []corev1.Pod{
		{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "pod-a",
				Namespace: "test-ns",
				Labels: map[string]string{
					"app": "test",
				},
			},
			Status: corev1.PodStatus{
				Phase: corev1.PodRunning,
			},
		},
	}

	tests := []struct {
		name    string
		service corev1.Service
		pods    []corev1.Pod
		want    types.HierarchyNode
	}{
		{
			name:    "service with matching pods",
			service: service,
			pods:    pods,
			want: types.HierarchyNode{
				Kind: types.ResourceKindService,
				Name: "test-service",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := createServiceNodeWithPods(tt.service, tt.pods)
			if got.Kind != tt.want.Kind || got.Name != tt.want.Name {
				t.Errorf("createServiceNodeWithPods() = %v, want %v", got, tt.want)
			}
			if len(got.Relatives) == 0 {
				t.Errorf("createServiceNodeWithPods() should have pod relatives")
			}
		})
	}
}

// Metadata extraction function tests (basic ones)
func Test_extractServiceMetadata(t *testing.T) {
	tests := []struct {
		name    string
		service corev1.Service
		want    types.ResourceMetadata
	}{
		{
			name: "basic service",
			service: corev1.Service{
				ObjectMeta: metav1.ObjectMeta{
					Labels: map[string]string{
						"app": "test",
					},
					Annotations: map[string]string{
						"constellation.kyledev.co/group": "my-group",
					},
				},
				Spec: corev1.ServiceSpec{
					Selector: map[string]string{
						"app": "test",
					},
					Type: corev1.ServiceTypeClusterIP,
					Ports: []corev1.ServicePort{
						{
							Port:       80,
							TargetPort: intstr.FromInt(8080),
						},
					},
				},
			},
			want: types.ResourceMetadata{
				Labels: map[string]string{
					"app": "test",
				},
				Selectors: map[string]string{
					"app": "test",
				},
				Group: "my-group",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := extractServiceMetadata(tt.service)
			if got.Group != tt.want.Group {
				t.Errorf("extractServiceMetadata() group = %v, want %v", got.Group, tt.want.Group)
			}
			if len(got.Labels) != len(tt.want.Labels) {
				t.Errorf("extractServiceMetadata() labels len = %v, want %v", len(got.Labels), len(tt.want.Labels))
			}
		})
	}
}

func Test_extractPodMetadata(t *testing.T) {
	tests := []struct {
		name string
		pod  corev1.Pod
		want types.ResourceMetadata
	}{
		{
			name: "basic pod",
			pod: corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Labels: map[string]string{
						"app": "test",
					},
					Annotations: map[string]string{
						"constellation.kyledev.co/display-name": "Test Pod",
					},
				},
				Status: corev1.PodStatus{
					Phase: corev1.PodRunning,
				},
			},
			want: types.ResourceMetadata{
				Labels: map[string]string{
					"app": "test",
				},
				DisplayName: "Test Pod",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := extractPodMetadata(tt.pod)
			if got.DisplayName != tt.want.DisplayName {
				t.Errorf("extractPodMetadata() display name = %v, want %v", got.DisplayName, tt.want.DisplayName)
			}
		})
	}
}
