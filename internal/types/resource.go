package types

import (
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/gateway-api/apis/v1beta1"
)

type ResourceKind string

const (
	ResourceKindNamespace ResourceKind = "Namespace"
	ResourceKindService   ResourceKind = "Service"
	ResourceKindPod       ResourceKind = "Pod"
	ResourceKindHTTPRoute ResourceKind = "HTTPRoute"
)

func (r ResourceKind) String() string {
	return string(r)
}

type ContainerPortInfo struct {
	Port     int32   `json:"port"`
	Name     *string `json:"name,omitempty"`
	Protocol *string `json:"protocol,omitempty"`
}

type ResourceMetadata struct {
	Hostnames       []string            `json:"hostnames,omitempty"`
	Selectors       map[string]string   `json:"selectors,omitempty"`
	Ports           []int32             `json:"ports,omitempty"`
	PortMappings    []string            `json:"port_mappings,omitempty"`
	TargetPorts     []int32             `json:"target_ports,omitempty"`
	TargetPortNames []string            `json:"target_port_names,omitempty"`
	Labels          map[string]string   `json:"labels,omitempty"`
	Phase           *string             `json:"phase,omitempty"`
	BackendRefs     []string            `json:"backend_refs,omitempty"`
	ServiceType     *string             `json:"service_type,omitempty"`
	ClusterIPs      []string            `json:"cluster_ips,omitempty"`
	ExternalIPs     []string            `json:"external_ips,omitempty"`
	PodIPs          []string            `json:"pod_ips,omitempty"`
	ContainerPorts  []ContainerPortInfo `json:"container_ports,omitempty"`
	Group           string              `json:"group,omitempty"`
	DisplayName     string              `json:"display_name,omitempty"`
	Ignore          bool                `json:"ignore,omitempty"`
}

type Resource struct {
	Kind      ResourceKind     `json:"kind"`
	Name      string           `json:"name"`
	Namespace string           `json:"namespace"`
	Metadata  ResourceMetadata `json:"metadata"`
	CreatedAt metav1.Time      `json:"created_at"`
}

// HierarchyNode represents a resource with its child resources
type HierarchyNode struct {
	Kind      ResourceKind    `json:"kind"`
	Name      string          `json:"name"`
	Namespace *string         `json:"namespace,omitempty"`
	Relatives []HierarchyNode `json:"relatives,omitempty"`
	// Embed all metadata fields directly
	Hostnames       []string            `json:"hostnames,omitempty"`
	Selectors       map[string]string   `json:"selectors,omitempty"`
	Ports           []int32             `json:"ports,omitempty"`
	PortMappings    []string            `json:"port_mappings,omitempty"`
	TargetPorts     []int32             `json:"target_ports,omitempty"`
	TargetPortNames []string            `json:"target_port_names,omitempty"`
	ContainerPorts  []ContainerPortInfo `json:"container_ports,omitempty"`
	Labels          map[string]string   `json:"labels,omitempty"`
	Phase           *string             `json:"phase,omitempty"`
	BackendRefs     []string            `json:"backend_refs,omitempty"`
	ServiceType     *string             `json:"service_type,omitempty"`
	ClusterIPs      []string            `json:"cluster_ips,omitempty"`
	ExternalIPs     []string            `json:"external_ips,omitempty"`
	PodIPs          []string            `json:"pod_ips,omitempty"`
	Group           string              `json:"group,omitempty"`
	DisplayName     string              `json:"display_name,omitempty"`
	Ignore          bool                `json:"ignore,omitempty"`
}

type ClusterState struct {
	Resources   map[string]Resource `json:"resources"`
	Connections []Connection        `json:"connections"`
}

type Connection struct {
	Source string `json:"source"`
	Target string `json:"target"`
}

type ResourceSpec interface {
	GetKind() ResourceKind
}

type NamespaceSpec struct{}

func (n NamespaceSpec) GetKind() ResourceKind { return ResourceKindNamespace }

type ServiceSpec struct {
	Spec *corev1.ServiceSpec
}

func (s ServiceSpec) GetKind() ResourceKind { return ResourceKindService }

type PodSpec struct {
	Spec *corev1.PodSpec
}

func (p PodSpec) GetKind() ResourceKind { return ResourceKindPod }

type HTTPRouteSpec struct {
	Spec *v1beta1.HTTPRouteSpec
}

func (h HTTPRouteSpec) GetKind() ResourceKind { return ResourceKindHTTPRoute }
