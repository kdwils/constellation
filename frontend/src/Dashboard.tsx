
import { useState, useEffect } from "react";
import type { ResourceNode } from "./ResourceNode";
import { NamespaceSidebar } from "./components/NamespaceSidebar";
import { NamespaceDetailView } from "./components/NamespaceDetailView";

export default function Dashboard() {
    const [data, setData] = useState<ResourceNode[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [selectedNamespace, setSelectedNamespace] = useState<string | null>(null);

    useEffect(() => {
        fetch("/state")
            .then(res => {
                if (!res.ok) {
                    throw new Error(`HTTP error! status: ${res.status}`);
                }
                return res.json();
            })
            .then((data) => {
                setData(data);
                setLoading(false);
                // Auto-select first namespace if available
                if (data.length > 0 && !selectedNamespace) {
                    setSelectedNamespace(data[0].name);
                }
            })
            .catch((err) => {
                setError(err.message);
                setLoading(false);
            });
    }, [selectedNamespace]);

    if (loading) {
        return (
            <div className="min-h-screen bg-gray-50 flex items-center justify-center">
                <div className="text-center">
                    <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
                    <p className="mt-4 text-gray-600">Loading cluster resources...</p>
                </div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="min-h-screen bg-gray-50 flex items-center justify-center">
                <div className="text-center">
                    <div className="text-red-600 text-6xl mb-4">⚠️</div>
                    <h2 className="text-2xl font-bold text-gray-900 mb-2">Error Loading Data</h2>
                    <p className="text-gray-600">{error}</p>
                </div>
            </div>
        );
    }

    const totalNamespaces = data.length;
    const totalResources = data.reduce((sum, namespace) => {
        return sum + countTotalResources(namespace);
    }, 0);

    const currentNamespace = selectedNamespace ? data.find(ns => ns.name === selectedNamespace) : null;

    return (
        <div className="min-h-screen bg-gray-50 flex flex-col">
            <header className="bg-white shadow-sm border-b border-gray-200 flex-shrink-0">
                <div className="w-full px-4 py-4">
                    <div className="flex items-center justify-between">
                        <div>
                            <h1 className="text-xl font-bold text-gray-800">
                                Constellation
                            </h1>
                            <p className="text-gray-600 text-sm">
                                Kubernetes Resource Relationships
                            </p>
                        </div>
                        <div className="text-right">
                            <div className="flex space-x-6">
                                <div className="text-center">
                                    <div className="text-lg font-bold text-gray-700">{totalNamespaces}</div>
                                    <div className="text-xs text-gray-500 uppercase tracking-wide font-medium">Namespaces</div>
                                </div>
                                <div className="text-center">
                                    <div className="text-lg font-bold text-gray-700">{totalResources}</div>
                                    <div className="text-xs text-gray-500 uppercase tracking-wide font-medium">Resources</div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </header>

            <div className="flex flex-1 overflow-hidden">
                {data.length > 0 ? (
                    <>
                        <NamespaceSidebar 
                            namespaces={data}
                            selectedNamespace={selectedNamespace}
                            onNamespaceSelect={setSelectedNamespace}
                        />
                        <div className="flex-1 flex">
                            {currentNamespace ? (
                                <NamespaceDetailView namespace={currentNamespace} />
                            ) : (
                                <div className="flex-1 flex items-center justify-center bg-white">
                                    <div className="text-center">
                                        <div className="text-gray-400 text-6xl mb-4">📋</div>
                                        <h2 className="text-xl font-semibold text-gray-900 mb-2">Select a Namespace</h2>
                                        <p className="text-gray-600">Choose a namespace from the sidebar to view its resources.</p>
                                    </div>
                                </div>
                            )}
                        </div>
                    </>
                ) : (
                    <div className="flex-1 flex items-center justify-center">
                        <div className="text-center">
                            <div className="text-gray-400 text-6xl mb-4">📦</div>
                            <h2 className="text-xl font-semibold text-gray-900 mb-2">No Resources Found</h2>
                            <p className="text-gray-600">No Kubernetes resources are currently being tracked.</p>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}

function countTotalResources(node: ResourceNode): number {
    if (!node.relatives) return 0;
    
    let count = node.relatives.length;
    for (const relative of node.relatives) {
        count += countTotalResources(relative);
    }
    return count;
}