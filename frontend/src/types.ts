export type Kind = "Namespace" | "Ingress" | "HTTPRoute" | "Service" | "Pod";

export interface ContainerPortInfo {
    port: number;
    name?: string;
    protocol?: string;
}

export interface HealthCheckEntry {
    timestamp: string;
    status: "healthy" | "unhealthy" | "unknown";
    latency: number;
    error?: string;
    url: string;
    method: string;
    response_code?: number;
}

export interface ServiceHealthInfo {
    service_name: string;
    namespace: string;
    last_check: string;
    status: "healthy" | "unhealthy" | "unknown";
    uptime: number;
    history: HealthCheckEntry[];
    url: string;
}

export interface ResourceNode {
    kind: Kind;
    name: string;
    namespace?: string;
    relatives?: ResourceNode[];
    health?: "Healthy" | "Degraded" | "Error" | "Unknown";
    health_info?: ServiceHealthInfo;
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
    cluster_ips?: string[];
    external_ips?: string[];
    pod_ips?: string[];
    group?: string;
    display_name?: string;
    ignore?: boolean;
}
