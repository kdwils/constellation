import type { ResourceNode } from "../ResourceNode";

interface NamespaceSidebarProps {
    namespaces: ResourceNode[];
    selectedNamespace: string | null;
    onNamespaceSelect: (namespace: string) => void;
}

interface NamespaceStats {
    pods: number;
    healthyPods: number;
    hasExternalRoutes: boolean;
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

export function NamespaceSidebar({ namespaces, selectedNamespace, onNamespaceSelect }: NamespaceSidebarProps) {
    return (
        <div className="w-80 bg-white border-r border-gray-200 flex flex-col h-screen">
            <div className="p-4 border-b border-gray-200 flex-shrink-0">
                <h2 className="text-lg font-semibold text-gray-900">Namespaces</h2>
                <p className="text-sm text-gray-600 mt-1">{namespaces.length} total</p>
            </div>
            
            <div className="flex-1 overflow-y-auto overflow-x-hidden min-h-0">
                <div className="p-2">
                    {namespaces.map((namespace) => {
                        const stats = getNamespaceStats(namespace);
                        const isSelected = selectedNamespace === namespace.name;
                        const healthPercent = stats.pods > 0 ? Math.round((stats.healthyPods / stats.pods) * 100) : 0;
                        
                        return (
                            <button
                                key={namespace.name}
                                onClick={() => onNamespaceSelect(namespace.name)}
                                className={`w-full p-4 rounded-lg text-left mb-2 transition-all duration-200 ${
                                    isSelected 
                                        ? 'bg-blue-50 border-2 border-blue-200 shadow-sm' 
                                        : 'bg-gray-50 hover:bg-gray-100 border-2 border-transparent'
                                }`}
                            >
                                <div className="flex items-center justify-between mb-2">
                                    <h3 className={`font-medium truncate ${
                                        isSelected ? 'text-blue-900' : 'text-gray-900'
                                    }`}>
                                        {namespace.name}
                                    </h3>
                                    {stats.hasExternalRoutes && (
                                        <div className="flex-shrink-0">
                                            <div className="w-2 h-2 bg-green-400 rounded-full" title="Has external routes"></div>
                                        </div>
                                    )}
                                </div>
                                
                                <div className="space-y-1 text-xs text-gray-600">
                                    <div className="flex justify-between">
                                        <span>Pods:</span>
                                        <div className="flex items-center space-x-2">
                                            <span className="font-medium">{stats.healthyPods}/{stats.pods}</span>
                                            {stats.pods > 0 && (
                                                <div className={`w-12 h-1.5 rounded-full ${
                                                    healthPercent === 100 ? 'bg-green-200' :
                                                    healthPercent > 50 ? 'bg-yellow-200' : 'bg-red-200'
                                                }`}>
                                                    <div 
                                                        className={`h-full rounded-full transition-all duration-300 ${
                                                            healthPercent === 100 ? 'bg-green-500' :
                                                            healthPercent > 50 ? 'bg-yellow-500' : 'bg-red-500'
                                                        }`}
                                                        style={{ width: `${healthPercent}%` }}
                                                    />
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                </div>
                            </button>
                        );
                    })}
                </div>
            </div>
            
            <div className="p-4 border-t border-gray-200 bg-gray-50 flex-shrink-0">
                <p className="text-xs text-gray-500 text-center">
                    Select a namespace to view details
                </p>
            </div>
        </div>
    );
}