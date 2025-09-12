import type { ResourceNode } from "../types";

export interface ResourceStats {
    totalResources: number;
    totalServices: number;
    totalPods: number;
    healthyPods: number;
    ingresses: number;
    httpRoutes: number;
    hasExternalRoutes: boolean;
    namespaces: Set<string>;
}

export interface GroupInfo {
    name: string;
    resources: ResourceNode[];
}

/**
 * Recursively counts total resources in a node hierarchy
 */
export function countTotalResources(node: ResourceNode): number {
    if (!node.relatives) return 0;

    let count = node.relatives.length;
    for (const relative of node.relatives) {
        count += countTotalResources(relative);
    }
    return count;
}

/**
 * Recursively counts resources by type and calculates health metrics
 */
function traverseResourcesForStats(nodes: ResourceNode[]): Omit<ResourceStats, 'namespaces'> {
    let totalResources = nodes.length;
    let totalServices = 0;
    let totalPods = 0;
    let healthyPods = 0;
    let ingresses = 0;
    let httpRoutes = 0;
    let hasExternalRoutes = false;

    for (const node of nodes) {
        switch (node.kind) {
            case "Ingress":
                ingresses++;
                hasExternalRoutes = true;
                break;
            case "HTTPRoute":
                httpRoutes++;
                hasExternalRoutes = true;
                break;
            case "Service":
                totalServices++;
                break;
            case "Pod":
                totalPods++;
                if (node.phase === "Running") {
                    healthyPods++;
                }
                break;
        }

        if (node.relatives) {
            const childStats = traverseResourcesForStats(node.relatives);
            totalResources += childStats.totalResources;
            totalServices += childStats.totalServices;
            totalPods += childStats.totalPods;
            healthyPods += childStats.healthyPods;
            ingresses += childStats.ingresses;
            httpRoutes += childStats.httpRoutes;
            hasExternalRoutes = hasExternalRoutes || childStats.hasExternalRoutes;
        }
    }

    return {
        totalResources,
        totalServices,
        totalPods,
        healthyPods,
        ingresses,
        httpRoutes,
        hasExternalRoutes,
    };
}

/**
 * Calculate comprehensive stats for a namespace
 */
export function calculateNamespaceStats(namespace: ResourceNode): ResourceStats {
    const namespaces = new Set<string>([namespace.name]);
    
    if (!namespace.relatives) {
        return {
            totalResources: 0,
            totalServices: 0,
            totalPods: 0,
            healthyPods: 0,
            ingresses: 0,
            httpRoutes: 0,
            hasExternalRoutes: false,
            namespaces,
        };
    }

    const stats = traverseResourcesForStats(namespace.relatives);
    
    return {
        ...stats,
        namespaces,
    };
}

/**
 * Calculate stats for a collection of resources (used for groups)
 */
export function calculateResourceCollectionStats(resources: ResourceNode[]): ResourceStats {
    const namespaces = new Set<string>();
    
    // Collect namespaces from all resources
    resources.forEach(resource => {
        if (resource.namespace) {
            namespaces.add(resource.namespace);
        }
    });

    const stats = traverseResourcesForStats(resources);
    
    return {
        ...stats,
        namespaces,
    };
}

/**
 * Calculate health percentage
 */
export function calculateHealthPercentage(healthyPods: number, totalPods: number): number {
    return totalPods > 0 ? Math.round((healthyPods / totalPods) * 100) : 0;
}

/**
 * Extract groups from namespaces
 */
export function extractGroups(namespaces: ResourceNode[]): GroupInfo[] {
    const groups = new Map<string, ResourceNode[]>();

    for (const namespace of namespaces) {
        if (namespace.relatives) {
            for (const resource of namespace.relatives) {
                if (resource.group) {
                    const resourceWithNamespace = {
                        ...resource,
                        namespace: namespace.name,
                        relatives: resource.relatives
                    };

                    if (!groups.has(resource.group)) {
                        groups.set(resource.group, []);
                    }
                    groups.get(resource.group)!.push(resourceWithNamespace);
                }
            }
        }
    }

    return Array.from(groups.entries())
        .map(([name, resources]) => ({ name, resources }))
        .sort((a, b) => a.name.localeCompare(b.name));
}

/**
 * Calculate total resources across all namespaces
 */
export function calculateTotalResourcesAcrossNamespaces(namespaces: ResourceNode[]): number {
    return namespaces.reduce((sum, namespace) => {
        return sum + countTotalResources(namespace);
    }, 0);
}