
interface NamespaceHeaderProps {
    name: string;
    resourceCount: number;
    isCollapsed: boolean;
    onToggle: () => void;
}

export function NamespaceHeader({ name, resourceCount, isCollapsed, onToggle }: NamespaceHeaderProps) {
    return (
        <button 
            onClick={onToggle}
            className="w-full p-4 bg-blue-50 hover:bg-blue-100 border-b border-blue-200 text-left transition-colors duration-200"
        >
            <div className="flex items-center justify-between">
                <span className="text-lg font-semibold text-blue-900">
                    🏷️ {name}
                </span>
                <div className="flex items-center space-x-3">
                    <span className="text-sm text-blue-700 bg-blue-200 px-2 py-1 rounded-full">
                        {resourceCount} resources
                    </span>
                    <div className="text-blue-600 text-xl">
                        {isCollapsed ? '▶' : '▼'}
                    </div>
                </div>
            </div>
        </button>
    );
}


