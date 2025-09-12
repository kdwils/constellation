import { useState } from "react";
import type { ReactNode } from "react";
import type { ResourceNode } from "../types";
import { ServiceBox } from "./Service";
import { PodBox } from "./Pod";
import { HttpRouteBox } from "./HttpRoute";
import { IngressBox } from "./Ingress";

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

interface CollapsibleSectionProps {
    title: string;
    isCollapsed: boolean;
    onToggle: () => void;
    children: ReactNode;
}

interface ResourceNodeWrapperProps {
    node: ResourceNode;
    level: number;
    children: ReactNode;
    serviceSelectors?: Record<string, string>;
    targetPorts?: number[];
    targetPortNames?: string[];
    backendRefs?: string[];
}

function CollapsibleSection({ title, isCollapsed, onToggle, children }: CollapsibleSectionProps) {
    return (
        <div className="bg-white rounded-lg border border-gray-200 overflow-hidden">
            <button
                onClick={onToggle}
                className="w-full px-6 py-4 bg-gray-50 hover:bg-gray-100 border-b border-gray-200 text-left transition-colors duration-200 flex items-center justify-between"
            >
                <h2 className="text-xl font-semibold text-gray-900">{title}</h2>
                <div className="text-gray-400 text-sm">
                    {isCollapsed ? '▶' : '▼'}
                </div>
            </button>

            <div className={`transition-all duration-300 ease-in-out overflow-hidden ${isCollapsed ? 'max-h-0 opacity-0' : 'max-h-[600px] opacity-100'
                }`}>
                <div className="p-6 overflow-y-auto max-h-[550px]">
                    {children}
                </div>
            </div>
        </div>
    );
}

function ResourceNodeWrapper({ node, level, children, serviceSelectors, targetPorts, targetPortNames, backendRefs }: ResourceNodeWrapperProps) {
    const hasChildren = node.relatives && node.relatives.length > 0;
    const shouldHaveBorder = level === 0;

    return (
        <div className={`${shouldHaveBorder ? 'border border-gray-200 rounded-lg p-4 bg-gray-50/50' : ''} space-y-4`}>
            {children}
            {hasChildren && (
                <div className="ml-6 space-y-4">
                    {renderResourceTree(node.relatives!, level + 1, serviceSelectors, targetPorts, targetPortNames, backendRefs)}
                </div>
            )}
        </div>
    );
}

function renderResourceTree(nodes: ResourceNode[], level: number = 0, serviceSelectors?: Record<string, string>, targetPorts?: number[], targetPortNames?: string[], backendRefs?: string[]): ReactNode[] {
    const collectBackendRefs = (relatives?: ResourceNode[]): string[] => {
        if (!relatives) return [];
        return relatives
            .filter(node => node.kind === "HTTPRoute")
            .flatMap(route => route.backend_refs || []);
    };

    return nodes.map((node) => {
        const key = `${node.kind}-${node.name}-${level}`;

        switch (node.kind) {
            case "Ingress":
                return (
                    <ResourceNodeWrapper key={key} node={node} level={level} serviceSelectors={serviceSelectors} targetPorts={targetPorts} targetPortNames={targetPortNames} backendRefs={backendRefs}>
                        <IngressBox name={node.name} />
                    </ResourceNodeWrapper>
                );

            case "HTTPRoute": {
                // Get the names of child services that match backend_refs
                const referencedServiceNames: string[] = [];
                if (node.backend_refs && node.relatives) {
                    node.backend_refs.forEach(ref => {
                        if (node.relatives!.some(relative => relative.kind === "Service" && relative.name === ref)) {
                            referencedServiceNames.push(ref);
                        }
                    });
                }

                return (
                    <ResourceNodeWrapper key={key} node={node} level={level} serviceSelectors={serviceSelectors} targetPorts={targetPorts} targetPortNames={targetPortNames} backendRefs={backendRefs}>
                        <HttpRouteBox name={node.name} hostnames={node.hostnames} backend_refs={node.backend_refs} referencedServiceNames={referencedServiceNames} />
                    </ResourceNodeWrapper>
                );
            }

            case "Service": {
                const currentBackendRefs = collectBackendRefs(nodes);
                const isTargetedByRoute = (backendRefs || currentBackendRefs).includes(node.name);

                const childContainerPorts: any[] = [];
                if (node.relatives) {
                    for (const childNode of node.relatives) {
                        if (childNode.kind === "Pod" && childNode.container_ports) {
                            childContainerPorts.push(...childNode.container_ports);
                        }
                    }
                }

                return (
                    <ResourceNodeWrapper
                        key={key}
                        node={node}
                        level={level}
                        serviceSelectors={node.selectors}
                        targetPorts={node.target_ports}
                        targetPortNames={node.target_port_names}
                        backendRefs={backendRefs || currentBackendRefs}
                    >
                        <ServiceBox
                            name={node.name}
                            selectors={node.selectors}
                            portMappings={node.port_mappings}
                            isTargetedByRoute={isTargetedByRoute}
                            serviceType={node.service_type}
                            clusterIps={node.cluster_ips}
                            externalIps={node.external_ips}
                            childContainerPorts={childContainerPorts}
                        />
                    </ResourceNodeWrapper>
                );
            }

            case "Pod":
                return (
                    <ResourceNodeWrapper key={key} node={node} level={level} serviceSelectors={serviceSelectors} targetPorts={targetPorts} targetPortNames={targetPortNames} backendRefs={backendRefs}>
                        <PodBox
                            name={node.name}
                            labels={node.labels}
                            containerPorts={node.container_ports}
                            serviceSelectors={serviceSelectors}
                            targetPorts={targetPorts}
                            targetPortNames={targetPortNames}
                            phase={node.phase}
                            podIps={node.pod_ips}
                        />
                    </ResourceNodeWrapper>
                );

            default:
                return <div key={key}></div>;
        }
    });
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
                                {renderResourceTree(namespace.relatives, 0)}
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