import { useState } from "react";
import type { ResourceNode } from "./ResourceNode";
import { NamespaceHeader } from "./resources/NamespaceHeader";
import { ServiceBox } from "./resources/Service";
import { PodBox } from "./resources/Pod";
import { HttpRouteBox } from "./resources/HttpRoute";
import { IngressBox } from "./resources/Ingress";

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
}

function ResourceNodeItem({ node, level = 0, serviceSelectors }: ResourceNodeItemProps) {
    const [isCollapsed, setIsCollapsed] = useState(true);

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
                <div className={`transition-all duration-300 ease-in-out overflow-hidden ${
                    isCollapsed ? 'max-h-0 opacity-0' : 'max-h-[600px] opacity-100'
                }`}>
                    <div className="p-4 space-y-6 overflow-y-auto max-h-[550px]">
                        {node.relatives && node.relatives.length > 0 ? (
                            node.relatives.map((childNode) => (
                                <div key={childNode.name} className="border border-gray-200 rounded-lg p-4 bg-gray-50/50 space-y-2">
                                    <ResourceNodeItem node={childNode} level={level + 1} serviceSelectors={serviceSelectors} />
                                </div>
                            ))
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
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} serviceSelectors={serviceSelectors} />
                ))}
            </div>
        );
    }

    if (node.kind === "HTTPRoute") {
        return (
            <div className="space-y-2">
                <HttpRouteBox name={node.name} hostnames={node.hostnames} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} serviceSelectors={serviceSelectors} />
                ))}
            </div>
        );
    }

    if (node.kind === "Service") {
        return (
            <div className="space-y-2">
                <ServiceBox name={node.name} selectors={node.selectors} ports={node.ports} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} serviceSelectors={node.selectors} />
                ))}
            </div>
        );
    }

    if (node.kind === "Pod") {
        return (
            <div className="space-y-2">
                <PodBox name={node.name} labels={node.labels} ports={node.ports} serviceSelectors={serviceSelectors} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} serviceSelectors={serviceSelectors} />
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
