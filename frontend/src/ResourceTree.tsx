import { useState } from "react";
import type { ResourceNode, ContainerPortInfo } from "./ResourceNode";
import { NamespaceHeader } from "./components/NamespaceHeader";
import { ServiceBox } from "./components/Service";
import { PodBox } from "./components/Pod";
import { HttpRouteBox } from "./components/HttpRoute";
import { IngressBox } from "./components/Ingress";

interface ResourceTreeProps {
    nodes: ResourceNode[];
}

export function ResourceTree({ nodes }: ResourceTreeProps) {
    return (
        <div className="space-y-4 w-full flex flex-col min-w-0">
            {nodes.map((node) => (
                <ResourceNodeItem key={node.kind + node.name} node={node} />
            ))}
        </div>
    );
}

interface ResourceNodeItemProps {
    node: ResourceNode;
    level?: number;
    serviceSelectors?: Record<string, string>;
    targetPorts?: number[];
    targetPortNames?: string[];
    backendRefs?: string[];
}

function ResourceNodeItem({ node, level = 0, serviceSelectors, targetPorts, targetPortNames, backendRefs }: ResourceNodeItemProps) {
    const [isCollapsed, setIsCollapsed] = useState(true);

    const collectBackendRefs = (relatives?: ResourceNode[]): string[] => {
        if (!relatives) return [];
        return relatives
            .filter(node => node.kind === "HTTPRoute")
            .flatMap(route => route.backend_refs || []);
    };

    if (node.kind === "Namespace") {
        const resourceCount = countTotalResources(node);

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
                            node.relatives.map((childNode) => {
                                const backendRefs = collectBackendRefs(node.relatives);
                                return (
                                    <div key={childNode.name} className="border border-gray-200 rounded-lg p-4 bg-gray-50/50 space-y-2">
                                        <ResourceNodeItem node={childNode} level={level + 1} serviceSelectors={serviceSelectors} targetPorts={targetPorts} targetPortNames={targetPortNames} backendRefs={backendRefs} />
                                    </div>
                                );
                            })
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

    if (node.kind === "Ingress") {
        return (
            <div className="space-y-2">
                <IngressBox name={node.name} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} serviceSelectors={serviceSelectors} targetPorts={targetPorts} targetPortNames={targetPortNames} backendRefs={backendRefs} />
                ))}
            </div>
        );
    }

    if (node.kind === "HTTPRoute") {
        return (
            <div className="space-y-2">
                <HttpRouteBox name={node.name} hostnames={node.hostnames} backend_refs={node.backend_refs} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} serviceSelectors={serviceSelectors} targetPorts={targetPorts} targetPortNames={targetPortNames} backendRefs={backendRefs} />
                ))}
            </div>
        );
    }

    if (node.kind === "Service") {
        const isTargetedByRoute = backendRefs?.includes(node.name) || false;
        
        const childContainerPorts: ContainerPortInfo[] = [];
        if (node.relatives) {
            for (const childNode of node.relatives) {
                if (childNode.kind === "Pod" && childNode.container_ports) {
                    childContainerPorts.push(...childNode.container_ports);
                }
            }
        }
        
        return (
            <div className="space-y-2">
                <ServiceBox name={node.name} selectors={node.selectors} portMappings={node.port_mappings} isTargetedByRoute={isTargetedByRoute} serviceType={node.service_type} clusterIps={node.cluster_ips} externalIps={node.external_ips} childContainerPorts={childContainerPorts} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} serviceSelectors={node.selectors} targetPorts={node.target_ports} targetPortNames={node.target_port_names} backendRefs={backendRefs} />
                ))}
            </div>
        );
    }

    if (node.kind === "Pod") {
        return (
            <div className="space-y-2">
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
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} serviceSelectors={serviceSelectors} targetPorts={targetPorts} targetPortNames={targetPortNames} backendRefs={backendRefs} />
                ))}
            </div>
        );
    }

    return null;
}

function countTotalResources(node: ResourceNode): number {
    if (!node.relatives) return 0;

    let count = node.relatives.length;
    for (const relative of node.relatives) {
        count += countTotalResources(relative);
    }
    return count;
}
