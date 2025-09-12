import type { ResourceNode } from "../types";
import { ViewModeDropdown } from "./ViewModeDropdown";
import { SidebarItem } from "./SidebarItem";
import { extractGroups, calculateNamespaceStats, calculateResourceCollectionStats } from "../utils/resourceStats";

interface NamespaceSidebarProps {
    namespaces: ResourceNode[];
    selectedNamespace: string | null;
    onNamespaceSelect: (namespace: string) => void;
    viewMode: 'namespace' | 'group';
    onViewModeChange: (mode: 'namespace' | 'group') => void;
    selectedGroup: string | null;
    onGroupSelect: (group: string) => void;
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
                            const stats = calculateNamespaceStats(namespace);
                            const isSelected = selectedNamespace === namespace.name;

                            return (
                                <SidebarItem
                                    key={namespace.name}
                                    name={`📁 ${namespace.name}`}
                                    isSelected={isSelected}
                                    onClick={() => onNamespaceSelect(namespace.name)}
                                    stats={{
                                        pods: stats.totalPods,
                                        healthyPods: stats.healthyPods
                                    }}
                                />
                            );
                        })}
                    </div>
                ) : (
                    <div className="p-2">
                        {groups.length > 0 ? groups.map((group) => {
                            const stats = calculateResourceCollectionStats(group.resources);
                            const isSelected = selectedGroup === group.name;

                            return (
                                <SidebarItem
                                    key={group.name}
                                    name={`🏷️ ${group.name}`}
                                    isSelected={isSelected}
                                    onClick={() => onGroupSelect(group.name)}
                                    stats={{
                                        pods: stats.totalPods,
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