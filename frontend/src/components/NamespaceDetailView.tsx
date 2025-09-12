import { useState } from "react";
import type { ResourceNode } from "../types";
import { ResourceTree } from "./ResourceTree";
import { CollapsibleSection } from "./CollapsibleSection";

interface NamespaceDetailViewProps {
    namespace: ResourceNode;
}

interface NamespaceOverview {
    totalServices: number;
    totalPods: number;
    healthyPods: number;
    ingresses: number;
    httpRoutes: number;
}

function getNamespaceOverview(namespace: ResourceNode): NamespaceOverview {
    let totalServices = 0;
    let totalPods = 0;
    let healthyPods = 0;
    let ingresses = 0;
    let httpRoutes = 0;

    function traverseResources(nodes: ResourceNode[]) {
        for (const node of nodes) {
            switch (node.kind) {
                case "Ingress":
                    ingresses++;
                    break;
                case "HTTPRoute":
                    httpRoutes++;
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
                traverseResources(node.relatives);
            }
        }
    }

    if (namespace.relatives) {
        traverseResources(namespace.relatives);
    }

    return { totalServices, totalPods, healthyPods, ingresses, httpRoutes };
}


export function NamespaceDetailView({ namespace }: NamespaceDetailViewProps) {
    const overview = getNamespaceOverview(namespace);
    const healthPercent = overview.totalPods > 0 ? Math.round((overview.healthyPods / overview.totalPods) * 100) : 0;
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
                                        {overview.healthyPods}/{overview.totalPods}
                                    </p>
                                </div>
                            </div>
                        </div>

                        <div className="bg-white rounded-lg border border-gray-200 p-6">
                            <div className="flex items-center">
                                <div className={`p-3 rounded-full ${healthPercent === 100 ? 'bg-green-100' :
                                        healthPercent > 50 ? 'bg-yellow-100' : 'bg-red-100'
                                    }`}>
                                    <div className={`w-6 h-6 ${healthPercent === 100 ? 'text-green-600' :
                                            healthPercent > 50 ? 'text-yellow-600' : 'text-red-600'
                                        }`}>
                                        {healthPercent === 100 ? '✅' : healthPercent > 50 ? '⚠️' : '❌'}
                                    </div>
                                </div>
                                <div className="ml-4">
                                    <p className="text-sm font-medium text-gray-600">Pod Health</p>
                                    <p className="text-2xl font-semibold text-gray-900">{healthPercent}%</p>
                                </div>
                            </div>
                        </div>
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
                            <div className="text-center py-12">
                                <div className="text-gray-400 text-4xl mb-4">📭</div>
                                <h3 className="text-lg font-medium text-gray-900 mb-2">No Resources Found</h3>
                                <p className="text-gray-600">This namespace doesn't contain any tracked resources.</p>
                            </div>
                        )}
                    </CollapsibleSection>
                </div>
            </div>
        </div>
    );
}