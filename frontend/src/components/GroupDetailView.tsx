import { useState } from "react";
import type { ResourceNode } from "../types";
import { ResourceTree } from "./ResourceTree";
import { CollapsibleSection } from "./CollapsibleSection";

interface GroupDetailViewProps {
    groupName: string;
    resources: ResourceNode[];
}

function countTotalResources(resources: ResourceNode[]): number {
    let count = resources.length;
    for (const resource of resources) {
        if (resource.relatives) {
            count += countTotalResources(resource.relatives);
        }
    }
    return count;
}

function getNamespaceStats(resources: ResourceNode[]): { namespaces: Set<string>; pods: number; healthyPods: number; hasExternalRoutes: boolean } {
    const namespaces = new Set<string>();
    let pods = 0;
    let healthyPods = 0;
    let hasExternalRoutes = false;

    function traverseResources(nodes: ResourceNode[]) {
        for (const node of nodes) {
            if (node.namespace) {
                namespaces.add(node.namespace);
            }

            switch (node.kind) {
                case "Ingress":
                case "HTTPRoute":
                    hasExternalRoutes = true;
                    break;
                case "Pod":
                    pods++;
                    if (node.phase === "Running") {
                        healthyPods++;
                    }
                    break;
            }

            if (node.relatives) {
                traverseResources(node.relatives);
            }
        }
    }

    traverseResources(resources);
    return { namespaces, pods, healthyPods, hasExternalRoutes };
}

export function GroupDetailView({ groupName, resources }: GroupDetailViewProps) {
    const totalResources = countTotalResources(resources);
    const stats = getNamespaceStats(resources);
    const healthPercent = stats.pods > 0 ? Math.round((stats.healthyPods / stats.pods) * 100) : 0;
    const [isResourcesCollapsed, setIsResourcesCollapsed] = useState(false);

    // Group resources by namespace for better organization
    const resourcesByNamespace = new Map<string, ResourceNode[]>();

    for (const resource of resources) {
        const ns = resource.namespace || 'default';
        if (!resourcesByNamespace.has(ns)) {
            resourcesByNamespace.set(ns, []);
        }
        resourcesByNamespace.get(ns)!.push(resource);
    }

    return (
        <div className="flex-1 flex flex-col bg-gray-50 overflow-hidden">
            <div className="bg-white border-b border-gray-200 px-6 py-4 flex-shrink-0">
                <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-3">
                        <div className="text-2xl">🏷️</div>
                        <div>
                            <h1 className="text-xl font-bold text-gray-900">{groupName}</h1>
                            <p className="text-sm text-gray-600">Custom Resource Group</p>
                        </div>
                    </div>

                    <div className="flex items-center space-x-6">
                        <div className="text-center">
                            <div className="text-sm font-semibold text-gray-900">{stats.namespaces.size}</div>
                            <div className="text-xs text-gray-500 uppercase tracking-wide">Namespaces</div>
                        </div>
                        <div className="text-center">
                            <div className="text-sm font-semibold text-gray-900">{totalResources}</div>
                            <div className="text-xs text-gray-500 uppercase tracking-wide">Resources</div>
                        </div>
                        {stats.pods > 0 && (
                            <div className="text-center">
                                <div className="text-sm font-semibold text-gray-900">{stats.healthyPods}/{stats.pods}</div>
                                <div className="text-xs text-gray-500 uppercase tracking-wide">Healthy Pods</div>
                            </div>
                        )}
                        {stats.hasExternalRoutes && (
                            <div className="text-center">
                                <div className="text-sm text-blue-600">🌐</div>
                                <div className="text-xs text-gray-500 uppercase tracking-wide">External</div>
                            </div>
                        )}
                    </div>
                </div>

                {stats.pods > 0 && (
                    <div className="mt-4">
                        <div className="flex items-center space-x-2">
                            <span className="text-xs text-gray-600">Pod Health:</span>
                            <div className="flex-1 max-w-xs">
                                <div className={`h-2 rounded-full ${healthPercent === 100 ? 'bg-green-200' :
                                        healthPercent > 50 ? 'bg-yellow-200' : 'bg-red-200'
                                    }`}>
                                    <div
                                        className={`h-full rounded-full transition-all duration-300 ${healthPercent === 100 ? 'bg-green-500' :
                                                healthPercent > 50 ? 'bg-yellow-500' : 'bg-red-500'
                                            }`}
                                        style={{ width: `${healthPercent}%` }}
                                    />
                                </div>
                            </div>
                            <span className="text-xs text-gray-600">{healthPercent}%</span>
                        </div>
                    </div>
                )}
            </div>

            <div className="flex-1 overflow-auto p-6">
                <div className="space-y-6">
                    <CollapsibleSection
                        title="Resource Relationships"
                        isCollapsed={isResourcesCollapsed}
                        onToggle={() => setIsResourcesCollapsed(!isResourcesCollapsed)}
                    >
                        {resources.length > 0 ? (
                            <div className="space-y-6">
                                {Array.from(resourcesByNamespace.entries())
                                    .sort(([a], [b]) => a.localeCompare(b))
                                    .map(([namespace, namespaceResources]) => (
                                        <ResourceTree key={namespace} nodes={namespaceResources} namespace={namespace} />
                                    ))}
                            </div>
                        ) : (
                            <div className="text-center py-12">
                                <div className="text-gray-400 text-4xl mb-4">🏷️</div>
                                <h3 className="text-lg font-medium text-gray-900 mb-2">No Resources Found</h3>
                                <p className="text-gray-600">This group doesn't contain any resources yet.</p>
                            </div>
                        )}
                    </CollapsibleSection>
                </div>
            </div>
        </div>
    );
}