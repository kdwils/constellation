import { useState } from "react";
import type { ResourceNode } from "../types";
import { ResourceTree } from "./ResourceTree";
import { CollapsibleSection } from "./CollapsibleSection";
import { calculateResourceCollectionStats, countTotalResources } from "../utils/resourceStats";
import { HealthIndicator } from "./shared/HealthIndicator";
import { EmptyStates } from "./shared/EmptyState";

interface GroupDetailViewProps {
    groupName: string;
    resources: ResourceNode[];
}

function countTotalResourcesInCollection(resources: ResourceNode[]): number {
    let count = resources.length;
    for (const resource of resources) {
        if (resource.relatives) {
            count += countTotalResources(resource);
        }
    }
    return count;
}

export function GroupDetailView({ groupName, resources }: GroupDetailViewProps) {
    const totalResources = countTotalResourcesInCollection(resources);
    const stats = calculateResourceCollectionStats(resources);
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
                        {stats.totalPods > 0 && (
                            <div className="text-center">
                                <div className="text-sm font-semibold text-gray-900">{stats.healthyPods}/{stats.totalPods}</div>
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

                {stats.totalPods > 0 && (
                    <div className="mt-4">
                        <HealthIndicator 
                            healthyPods={stats.healthyPods} 
                            totalPods={stats.totalPods} 
                            variant="bar" 
                        />
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
                            <EmptyStates.NoResources />
                        )}
                    </CollapsibleSection>
                </div>
            </div>
        </div>
    );
}