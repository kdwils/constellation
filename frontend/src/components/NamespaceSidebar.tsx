import type { ResourceNode } from "../types";
import { ViewModeDropdown } from "./ViewModeDropdown";
import { SidebarItem } from "./SidebarItem";

interface NamespaceSidebarProps {
    namespaces: ResourceNode[];
    selectedNamespace: string | null;
    onNamespaceSelect: (namespace: string) => void;
    viewMode: 'namespace' | 'group';
    onViewModeChange: (mode: 'namespace' | 'group') => void;
    selectedGroup: string | null;
    onGroupSelect: (group: string) => void;
}

interface NamespaceStats {
    pods: number;
    healthyPods: number;
    hasExternalRoutes: boolean;
}

interface GroupInfo {
    name: string;
    resources: ResourceNode[];
}

interface GroupStats {
    totalResources: number;
    pods: number;
    healthyPods: number;
    hasExternalRoutes: boolean;
    namespaces: Set<string>;
}

function getNamespaceStats(namespace: ResourceNode): NamespaceStats {
    let pods = 0;
    let healthyPods = 0;
    let hasExternalRoutes = false;

    function traverseResources(nodes: ResourceNode[]) {
        for (const node of nodes) {
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

    if (namespace.relatives) {
        traverseResources(namespace.relatives);
    }

    return { pods, healthyPods, hasExternalRoutes };
}

function extractGroups(namespaces: ResourceNode[]): GroupInfo[] {
    const groups = new Map<string, ResourceNode[]>();

    // Collect all resources from all namespaces
    for (const namespace of namespaces) {
        if (namespace.relatives) {
            for (const resource of namespace.relatives) {
                if (resource.group) {
                    // When we find a resource with a group annotation, include its full hierarchy
                    const resourceWithNamespace = {
                        ...resource,
                        namespace: namespace.name,
                        // Preserve all relatives (children) even if they don't have group annotations
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

    // Convert to array and sort by group name
    return Array.from(groups.entries())
        .map(([name, resources]) => ({ name, resources }))
        .sort((a, b) => a.name.localeCompare(b.name));
}

function getGroupStats(group: GroupInfo): GroupStats {
    let totalResources = group.resources.length;
    let pods = 0;
    let healthyPods = 0;
    let hasExternalRoutes = false;
    const namespaces = new Set<string>();

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
                totalResources += node.relatives.length;
                traverseResources(node.relatives);
            }
        }
    }

    traverseResources(group.resources);

    return { totalResources, pods, healthyPods, hasExternalRoutes, namespaces };
}

export function NamespaceSidebar({ namespaces, selectedNamespace, onNamespaceSelect, viewMode, onViewModeChange, selectedGroup, onGroupSelect }: NamespaceSidebarProps) {
    const groups = extractGroups(namespaces);
    return (
        <div className="w-80 bg-white border-r border-gray-200 flex flex-col h-screen">
            <div className="p-4 border-b border-gray-200 flex-shrink-0">
                <div className="flex items-center justify-between mb-3">
                    <h2 className="text-lg font-semibold text-gray-900">Navigation</h2>
                </div>

                <div className="mb-3">
                    <ViewModeDropdown
                        value={viewMode}
                        onChange={onViewModeChange}
                    />
                </div>

                {viewMode === 'namespace' ? (
                    <p className="text-sm text-gray-600">{namespaces.length} namespaces</p>
                ) : (
                    <p className="text-sm text-gray-600">{groups.length} groups</p>
                )}
            </div>

            <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0">
                {viewMode === 'namespace' ? (
                    <div className="p-2">
                        {namespaces.map((namespace) => {
                            const stats = getNamespaceStats(namespace);
                            const isSelected = selectedNamespace === namespace.name;

                            return (
                                <SidebarItem
                                    key={namespace.name}
                                    name={`📁 ${namespace.name}`}
                                    isSelected={isSelected}
                                    onClick={() => onNamespaceSelect(namespace.name)}
                                    stats={{
                                        pods: stats.pods,
                                        healthyPods: stats.healthyPods
                                    }}
                                />
                            );
                        })}
                    </div>
                ) : (
                    <div className="p-2">
                        {groups.length > 0 ? groups.map((group) => {
                            const stats = getGroupStats(group);
                            const isSelected = selectedGroup === group.name;

                            return (
                                <SidebarItem
                                    key={group.name}
                                    name={`🏷️ ${group.name}`}
                                    isSelected={isSelected}
                                    onClick={() => onGroupSelect(group.name)}
                                    stats={{
                                        pods: stats.pods,
                                        healthyPods: stats.healthyPods
                                    }}
                                />
                            );
                        }) : (
                            <div className="p-4 text-center text-gray-500">
                                <p className="text-sm">
                                    No custom groups found. Add annotations to your resources to create groups.
                                </p>
                            </div>
                        )}
                    </div>
                )}
            </div>

            <div className="p-4 border-t border-gray-200 bg-gray-50 flex-shrink-0">
                <p className="text-xs text-gray-500 text-center">
                    {viewMode === 'namespace'
                        ? 'Select a namespace to view details'
                        : 'Select a group to view details'
                    }
                </p>
            </div>
        </div>
    );
}