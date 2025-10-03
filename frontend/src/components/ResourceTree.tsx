import { useState } from "react";
import type { ResourceNode, ContainerPortInfo } from "../types";
import { NamespaceHeader } from "./NamespaceHeader";
import { ServiceBox } from "./Service";
import { PodBox } from "./Pod";
import { HttpRouteBox } from "./HttpRoute";
import { IngressBox } from "./Ingress";

interface ResourceTreeProps {
    nodes: ResourceNode[];
    namespace?: string;
}

interface ResourceWrapperProps {
    level: number;
    children: React.ReactNode;
}

function ResourceWrapper({ level, children }: ResourceWrapperProps) {
    const isRootLevel = level === 0;
    return (
        <div className={`${isRootLevel ? 'border border-gray-200 rounded-lg p-4 bg-gray-50/50' : ''} space-y-2`}>
            {children}
        </div>
    );
}

function countTotalResources(nodes: ResourceNode[]): number {
    let count = nodes.length;
    for (const node of nodes) {
        if (node.relatives) {
            count += countTotalResources(node.relatives);
        }
    }
    return count;
}

export function ResourceTree({ nodes, namespace }: ResourceTreeProps) {
    const collectBackendRefs = (relatives?: ResourceNode[]): string[] => {
        if (!relatives) return [];
        return relatives
            .filter(node => node.kind === "HTTPRoute")
            .flatMap(route => route.backend_refs || []);
    };

    const renderNodes = (nodeList: ResourceNode[], level: number = 0, serviceSelectors?: Record<string, string>, targetPorts?: number[], targetPortNames?: string[], backendRefs?: string[]): React.ReactNode[] => {
        return nodeList.map((node) => {
            const key = `${node.kind}-${node.name}-${level}`;
            
            switch (node.kind) {
                case "Namespace":
                    return (
                        <NamespaceNodeItem 
                            key={key}
                            node={node} 
                            level={level}
                        />
                    );
                    
                case "Ingress":
                    return (
                        <ResourceWrapper key={key} level={level}>
                            <IngressBox name={node.name} />
                            {node.relatives && node.relatives.length > 0 && (
                                <div className="ml-6 space-y-4">
                                    {renderNodes(node.relatives, level + 1, serviceSelectors, targetPorts, targetPortNames, backendRefs)}
                                </div>
                            )}
                        </ResourceWrapper>
                    );
                    
                case "HTTPRoute": {
                    const referencedServiceNames: string[] = [];
                    if (node.backend_refs && node.relatives) {
                        node.backend_refs.forEach(ref => {
                            if (node.relatives!.some(relative => relative.kind === "Service" && relative.name === ref)) {
                                referencedServiceNames.push(ref);
                            }
                        });
                    }
                    
                    return (
                        <ResourceWrapper key={key} level={level}>
                            <HttpRouteBox name={node.name} hostnames={node.hostnames} backend_refs={node.backend_refs} referencedServiceNames={referencedServiceNames} />
                            {node.relatives && node.relatives.length > 0 && (
                                <div className="ml-6 space-y-4">
                                    {renderNodes(node.relatives, level + 1, serviceSelectors, targetPorts, targetPortNames, backendRefs)}
                                </div>
                            )}
                        </ResourceWrapper>
                    );
                }
                
                case "Service": {
                    const currentBackendRefs = collectBackendRefs(nodeList);
                    const isTargetedByRoute = (backendRefs || currentBackendRefs).includes(node.name);
                    
                    const childContainerPorts: ContainerPortInfo[] = [];
                    if (node.relatives) {
                        for (const childNode of node.relatives) {
                            if (childNode.kind === "Pod" && childNode.container_ports) {
                                childContainerPorts.push(...childNode.container_ports);
                            }
                        }
                    }
                    
                    return (
                        <ResourceWrapper key={key} level={level}>
                            <ServiceBox
                                name={node.name}
                                selectors={node.selectors}
                                portMappings={node.port_mappings}
                                isTargetedByRoute={isTargetedByRoute}
                                serviceType={node.service_type}
                                clusterIps={node.cluster_ips}
                                externalIps={node.external_ips}
                                childContainerPorts={childContainerPorts}
                                healthInfo={node.health_info}
                            />
                            {node.relatives && node.relatives.length > 0 && (
                                <div className="ml-6 space-y-4">
                                    {renderNodes(node.relatives, level + 1, node.selectors, node.target_ports, node.target_port_names, backendRefs || currentBackendRefs)}
                                </div>
                            )}
                        </ResourceWrapper>
                    );
                }
                
                case "Pod":
                    return (
                        <ResourceWrapper key={key} level={level}>
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
                            {node.relatives && node.relatives.length > 0 && (
                                <div className="ml-6 space-y-4">
                                    {renderNodes(node.relatives, level + 1, serviceSelectors, targetPorts, targetPortNames, backendRefs)}
                                </div>
                            )}
                        </ResourceWrapper>
                    );
                    
                default:
                    return null;
            }
        });
    };

    const content = (
        <div className="space-y-4 w-full flex flex-col min-w-0">
            {renderNodes(nodes)}
        </div>
    );

    if (namespace) {
        return (
            <div className="space-y-6">
                <div className="space-y-4">
                    <div className="flex items-center space-x-2 mb-3">
                        <h2 className="text-lg font-medium text-gray-800">
                            📁 {namespace}
                        </h2>
                        <span className="text-sm text-gray-500">
                            ({countTotalResources(nodes)} {countTotalResources(nodes) === 1 ? 'resource' : 'resources'})
                        </span>
                    </div>

                    <div className="space-y-3">
                        {content}
                    </div>
                </div>
            </div>
        );
    }

    return content;
}

interface NamespaceNodeItemProps {
    node: ResourceNode;
    level: number;
}

function NamespaceNodeItem({ node }: NamespaceNodeItemProps) {
    const [isCollapsed, setIsCollapsed] = useState(true);

    if (node.kind !== "Namespace") {
        return null;
    }

    const resourceCount = countTotalResources(node.relatives || []);

    return (
        <div className="border border-gray-200 rounded-lg shadow-sm bg-white overflow-hidden block w-full">
            <NamespaceHeader
                name={node.name}
                resourceCount={resourceCount}
                isCollapsed={isCollapsed}
                onToggle={() => setIsCollapsed(!isCollapsed)}
            />
            <div className={`transition-all duration-300 ease-in-out overflow-hidden ${isCollapsed ? 'max-h-0 opacity-0' : 'max-h-[600px] opacity-100'
                }`}>
                <div className="p-4 space-y-6 overflow-y-auto max-h-[550px]">
                    {node.relatives && node.relatives.length > 0 ? (
                        <div className="space-y-4">
                            <ResourceTree nodes={node.relatives} />
                        </div>
                    ) : (
                        <div className="text-gray-500 italic text-center py-4">
                            No connected resources found
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

