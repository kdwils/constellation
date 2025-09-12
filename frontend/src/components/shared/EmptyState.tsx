interface EmptyStateProps {
    icon: string;
    title: string;
    description: string;
    children?: React.ReactNode;
    size?: 'sm' | 'md' | 'lg';
}

export function EmptyState({ 
    icon, 
    title, 
    description, 
    children, 
    size = 'md' 
}: EmptyStateProps) {
    const sizeClasses = {
        sm: {
            container: 'py-8',
            icon: 'text-3xl mb-2',
            title: 'text-lg font-medium text-gray-900 mb-1',
            description: 'text-sm text-gray-600'
        },
        md: {
            container: 'py-12',
            icon: 'text-4xl mb-4',
            title: 'text-lg font-medium text-gray-900 mb-2',
            description: 'text-gray-600'
        },
        lg: {
            container: 'py-16',
            icon: 'text-6xl mb-4',
            title: 'text-xl font-semibold text-gray-900 mb-2',
            description: 'text-gray-600'
        }
    };
    
    const classes = sizeClasses[size];
    
    return (
        <div className={`text-center ${classes.container}`}>
            <div className={`text-gray-400 ${classes.icon}`}>{icon}</div>
            <h3 className={classes.title}>{title}</h3>
            <p className={classes.description}>{description}</p>
            {children && (
                <div className="mt-6">
                    {children}
                </div>
            )}
        </div>
    );
}

// Preset empty states for common scenarios
export const EmptyStates = {
    NoNamespaces: () => (
        <EmptyState
            icon="📋"
            title="No Namespaces Found"
            description="Deploy something to get started."
        />
    ),
    
    NoResources: () => (
        <EmptyState
            icon="📭"
            title="No Resources Found"
            description="This namespace doesn't contain any tracked resources."
        />
    ),
    
    NoGroups: () => (
        <EmptyState
            icon="🏷️"
            title="No Custom Groups Found"
            description="Add annotations to your Kubernetes resources to organize them into custom groups."
        >
            <div className="bg-gray-50 rounded-lg p-6 text-left max-w-2xl mx-auto">
                <h4 className="text-lg font-medium text-gray-900 mb-4">Available Annotations</h4>
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
        </EmptyState>
    ),
    
    SelectNamespace: ({ hasNamespaces }: { hasNamespaces: boolean }) => (
        <EmptyState
            icon="📋"
            title={hasNamespaces ? "Select a Namespace" : "No Namespaces Found"}
            description={hasNamespaces 
                ? "Choose a namespace from the sidebar to view its resources."
                : "Deploy something to get started."
            }
        />
    ),
    
    SelectGroup: ({ hasGroups }: { hasGroups: boolean }) => (
        <EmptyState
            icon="🏷️"
            title={hasGroups ? "Select a Group" : "No Custom Groups Found"}
            description={hasGroups 
                ? "Choose a group from the sidebar to view its resources."
                : "Add annotations to your Kubernetes resources to organize them into custom groups."
            }
        />
    ),
    
    NoClusterResources: () => (
        <EmptyState
            icon="📦"
            title="No Resources Found"
            description="No Kubernetes resources are currently being tracked."
            size="lg"
        />
    )
};