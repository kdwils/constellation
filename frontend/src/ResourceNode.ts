export type Kind = "Namespace" | "Ingress" | "HTTPRoute" | "Service" | "Pod";

export interface ResourceNode {
    kind: Kind;
    name: string;
    namespace?: string;
    relatives?: ResourceNode[];
    health?: "Healthy" | "Degraded" | "Error" | "Unknown";
    hostnames?: string[];
    selectors?: Record<string, string>;
    ports?: number[];
    labels?: Record<string, string>;
    phase?: string;
}
