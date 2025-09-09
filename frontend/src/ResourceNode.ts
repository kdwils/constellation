export type Kind = "Namespace" | "Ingress" | "HTTPRoute" | "Service" | "Pod";

export interface ContainerPortInfo {
    port: number;
    name?: string;
    protocol?: string;
}

export interface ResourceNode {
    kind: Kind;
    name: string;
    namespace?: string;
    relatives?: ResourceNode[];
    health?: "Healthy" | "Degraded" | "Error" | "Unknown";
    hostnames?: string[];
    selectors?: Record<string, string>;
    ports?: number[];
    port_mappings?: string[];
    target_ports?: number[];
    target_port_names?: string[];
    container_ports?: ContainerPortInfo[];
    labels?: Record<string, string>;
    phase?: string;
    backend_refs?: string[];
    service_type?: string;
    cluster_ip?: string;
    external_ips?: string[];
    pod_ip?: string;
}
