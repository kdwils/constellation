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
        <div className="space-y-4">
            {nodes.map((node) => (
                <ResourceNodeItem key={node.kind + node.name} node={node} />
            ))}
        </div>
    );
}

interface ResourceNodeItemProps {
    node: ResourceNode;
    level?: number;
}

function ResourceNodeItem({ node, level = 0 }: ResourceNodeItemProps) {
    const [isCollapsed, setIsCollapsed] = useState(true);

    if (node.kind === "Namespace") {
        const resourceCount = countTotalResources(node);

        return (
            <div className="border rounded-lg shadow-lg bg-white overflow-hidden">
                <NamespaceHeader
                    name={node.name}
                    resourceCount={resourceCount}
                    isCollapsed={isCollapsed}
                    onToggle={() => setIsCollapsed(!isCollapsed)}
                />
                {!isCollapsed && (
                    <div className="p-4 space-y-3">
                        {node.relatives && node.relatives.length > 0 ? (
                            node.relatives.map((childNode, index) => (
                                <div key={childNode.name} className="space-y-2">
                                    <ResourceNodeItem node={childNode} level={level + 1} />
                                    {index < node.relatives!.length - 1 && (
                                        <div className="border-b border-gray-100"></div>
                                    )}
                                </div>
                            ))
                        ) : (
                            <div className="text-gray-500 italic text-center py-4">
                                No connected resources found
                            </div>
                        )}
                    </div>
                )}
            </div>
        );
    }

    if (node.kind === "Ingress") {
        return (
            <div className="space-y-2">
                <IngressBox name={node.name} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} />
                ))}
            </div>
        );
    }

    if (node.kind === "HTTPRoute") {
        return (
            <div className="space-y-2">
                <HttpRouteBox name={node.name} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} />
                ))}
            </div>
        );
    }

    if (node.kind === "Service") {
        return (
            <div className="space-y-2">
                <ServiceBox name={node.name} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} />
                ))}
            </div>
        );
    }

    if (node.kind === "Pod") {
        return (
            <div className="space-y-2">
                <PodBox name={node.name} />
                {node.relatives && node.relatives.map((childNode) => (
                    <ResourceNodeItem key={childNode.name} node={childNode} level={level + 1} />
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
