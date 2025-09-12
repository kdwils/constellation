import { useState } from "react";
import type { ResourceNode } from "../types";
import { ResourceTree } from "./ResourceTree";
import { CollapsibleSection } from "./CollapsibleSection";
import { calculateNamespaceStats } from "../utils/resourceStats";
import { HealthIndicator } from "./shared/HealthIndicator";
import { EmptyStates } from "./shared/EmptyState";

interface NamespaceDetailViewProps {
    namespace: ResourceNode;
}



export function NamespaceDetailView({ namespace }: NamespaceDetailViewProps) {
    const stats = calculateNamespaceStats(namespace);
    const [isResourcesCollapsed, setIsResourcesCollapsed] = useState(false);

    return (
        <div className="flex-1 overflow-y-auto bg-gray-50">
            <div className="w-full h-full">
                <div className="bg-white border-b border-gray-200 px-6 py-4">
                    <h1 className="text-3xl font-bold text-gray-900 mb-2">{namespace.name}</h1>
                    <p className="text-gray-600">Namespace resource overview and relationships</p>
                </div>

                <div className="px-6 py-4 space-y-6">
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                        <div className="bg-white rounded-lg border border-gray-200 p-6">
                            <div className="flex items-center">
                                <div className="p-3 rounded-full bg-blue-100">
                                    <div className="w-6 h-6 text-blue-600">📦</div>
                                </div>
                                <div className="ml-4">
                                    <p className="text-sm font-medium text-gray-600">Running Pods</p>
                                    <p className="text-2xl font-semibold text-gray-900">
                                        {stats.healthyPods}/{stats.totalPods}
                                    </p>
                                </div>
                            </div>
                        </div>

                        <HealthIndicator 
                            healthyPods={stats.healthyPods} 
                            totalPods={stats.totalPods} 
                            variant="card" 
                        />
                    </div>

                    <CollapsibleSection
                        title="Resource Relationships"
                        isCollapsed={isResourcesCollapsed}
                        onToggle={() => setIsResourcesCollapsed(!isResourcesCollapsed)}
                    >
                        {namespace.relatives && namespace.relatives.length > 0 ? (
                            <div className="space-y-4">
                                <ResourceTree nodes={namespace.relatives} namespace={namespace.name} />
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