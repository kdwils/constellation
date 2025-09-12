
import { useState, useEffect } from "react";
import type { ResourceNode } from "./types";
import { NamespaceSidebar } from "./components/NamespaceSidebar";
import { NamespaceDetailView } from "./components/NamespaceDetailView";
import { GroupDetailView } from "./components/GroupDetailView";

export default function Dashboard() {
    const [data, setData] = useState<ResourceNode[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [selectedNamespace, setSelectedNamespace] = useState<string | null>(null);
    const [connectionStatus, setConnectionStatus] = useState<'connecting' | 'connected' | 'disconnected'>('connecting');
    const [viewMode, setViewMode] = useState<'namespace' | 'group'>('namespace');
    const [selectedGroup, setSelectedGroup] = useState<string | null>(null);

    useEffect(() => {
        let eventSource: WebSocket | null = null;
        let retryTimeout: ReturnType<typeof setTimeout>;
        let healthCheckTimeout: ReturnType<typeof setTimeout>;

        const checkHealth = async (): Promise<boolean> => {
            try {
                const response = await fetch('/healthz');
                return response.ok;
            } catch {
                return false;
            }
        };

        const createConnection = async () => {
            const isHealthy = await checkHealth();

            if (!isHealthy) {
                setError('Server not ready. Waiting for Kubernetes resources...');
                healthCheckTimeout = setTimeout(() => {
                    setConnectionStatus('connecting');
                    createConnection();
                }, 2000);
                return;
            }

            const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const wsUrl = `${protocol}//${window.location.host}/state/stream`;
            eventSource = new WebSocket(wsUrl);

            eventSource.onopen = () => {
                setConnectionStatus('connected');
                setError(null);
            };

            eventSource.onmessage = (event) => {
                try {
                    const newData = JSON.parse(event.data);

                    setData(newData);
                    setLoading(false);

                    setSelectedNamespace(current => {
                        if (newData.length > 0 && !current) {
                            return newData[0].name;
                        }
                        return current;
                    });
                } catch (err) {
                    console.error('Failed to parse WebSocket data:', err);
                    setError('Failed to parse server data');
                }
            };

            eventSource.onerror = (err) => {
                console.error('WebSocket connection error:', err);
                setConnectionStatus('disconnected');
                setError('Connection to server lost. Retrying...');
                eventSource?.close();

                retryTimeout = setTimeout(() => {
                    setConnectionStatus('connecting');
                    createConnection();
                }, 5000);
            };

            eventSource.onclose = () => {
                if (connectionStatus !== 'disconnected') {
                    setConnectionStatus('disconnected');
                    setError('Connection to server lost. Retrying...');

                    retryTimeout = setTimeout(() => {
                        setConnectionStatus('connecting');
                        createConnection();
                    }, 5000);
                }
            };
        };

        createConnection();

        return () => {
            clearTimeout(retryTimeout);
            clearTimeout(healthCheckTimeout);
            eventSource?.close();
        };
    }, []);

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
    
    // Extract groups and find current group - preserve full hierarchies
    const extractGroups = (namespaces: ResourceNode[]) => {
        const groups = new Map<string, ResourceNode[]>();
        
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
        
        return groups;
    };
    
    const groups = extractGroups(data);
    const currentGroupResources = selectedGroup && groups.has(selectedGroup) ? groups.get(selectedGroup)! : null;
    
    // Handle view mode changes - clear selection when switching modes
    const handleViewModeChange = (mode: 'namespace' | 'group') => {
        setViewMode(mode);
        if (mode === 'namespace') {
            setSelectedGroup(null);
            // Auto-select first namespace if available
            if (data.length > 0 && !selectedNamespace) {
                setSelectedNamespace(data[0].name);
            }
        } else {
            setSelectedNamespace(null);
            // Auto-select first group if available
            if (groups.size > 0 && !selectedGroup) {
                setSelectedGroup(Array.from(groups.keys())[0]);
            }
        }
    };

    const getConnectionIndicator = () => {
        switch (connectionStatus) {
            case 'connected':
                return <div className="w-2 h-2 bg-green-500 rounded-full" title="Connected" />;
            case 'connecting':
                return <div className="w-2 h-2 bg-yellow-500 rounded-full animate-pulse" title="Connecting" />;
            case 'disconnected':
                return <div className="w-2 h-2 bg-red-500 rounded-full" title="Disconnected" />;
        }
    };

    return (
        <div className="min-h-screen bg-gray-50 flex flex-col">
            <header className="bg-white shadow-sm border-b border-gray-200 flex-shrink-0">
                <div className="w-full px-4 py-4">
                    <div className="flex items-center justify-between">
                        <div>
                            <div className="flex items-center gap-2">
                                <h1 className="text-xl font-bold text-gray-800">
                                    Constellation
                                </h1>
                                {getConnectionIndicator()}
                            </div>
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
                            viewMode={viewMode}
                            onViewModeChange={handleViewModeChange}
                            selectedGroup={selectedGroup}
                            onGroupSelect={setSelectedGroup}
                        />
                        <div className="flex-1 flex">
                            {viewMode === 'namespace' ? (
                                currentNamespace ? (
                                    <NamespaceDetailView namespace={currentNamespace} />
                                ) : (
                                    <div className="flex-1 bg-white p-6">
                                        <div className="text-center max-w-2xl mx-auto">
                                            <div className="text-gray-400 text-6xl mb-4">📋</div>
                                            <h2 className="text-xl font-semibold text-gray-900 mb-2">
                                                {data.length > 0 ? 'Select a Namespace' : 'No Namespaces Found'}
                                            </h2>
                                            <p className="text-gray-600">
                                                {data.length > 0 
                                                    ? 'Choose a namespace from the sidebar to view its resources.'
                                                    : 'Deploy something to get started.'
                                                }
                                            </p>
                                        </div>
                                    </div>
                                )
                            ) : (
                                currentGroupResources && selectedGroup ? (
                                    <GroupDetailView groupName={selectedGroup} resources={currentGroupResources} />
                                ) : (
                                    <div className="flex-1 bg-white p-6">
                                        <div className="text-center max-w-2xl mx-auto">
                                            <h2 className="text-xl font-semibold text-gray-900 mb-2">
                                                {groups.size > 0 ? 'Select a Group' : 'No Custom Groups Found'}
                                            </h2>
                                            <p className="text-gray-600 mb-6">
                                                {groups.size > 0 
                                                    ? 'Choose a group from the sidebar to view its resources.'
                                                    : 'Add annotations to your Kubernetes resources to organize them into custom groups.'
                                                }
                                            </p>
                                            
                                            {groups.size === 0 && (
                                                <div className="bg-gray-50 rounded-lg p-6 text-left">
                                                    <h3 className="text-lg font-medium text-gray-900 mb-4">Available Annotations</h3>
                                                    <div className="space-y-4">
                                                        <div className="border-l-4 border-blue-400 pl-4">
                                                            <code className="bg-blue-100 text-blue-800 px-2 py-1 rounded text-sm font-mono">
                                                                constellation.kyledev.co/group
                                                            </code>
                                                            <p className="text-gray-600 mt-2">
                                                                Group resources together by adding this annotation with a group name as the value. 
                                                                Resources with the same group name will appear together in the Groups view.
                                                            </p>
                                                        </div>
                                                        
                                                        <div className="border-l-4 border-green-400 pl-4">
                                                            <code className="bg-green-100 text-green-800 px-2 py-1 rounded text-sm font-mono">
                                                                constellation.kyledev.co/display-name
                                                            </code>
                                                            <p className="text-gray-600 mt-2">
                                                                Override the default resource name with a custom display name. 
                                                                Useful for showing friendly names instead of technical resource names.
                                                            </p>
                                                        </div>
                                                        
                                                        <div className="border-l-4 border-red-400 pl-4">
                                                            <code className="bg-red-100 text-red-800 px-2 py-1 rounded text-sm font-mono">
                                                                constellation.kyledev.co/ignore
                                                            </code>
                                                            <p className="text-gray-600 mt-2">
                                                                Hide resources from the dashboard by setting this annotation to "true". 
                                                                Useful for internal or maintenance resources that shouldn't be displayed.
                                                            </p>
                                                        </div>
                                                    </div>
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                )
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