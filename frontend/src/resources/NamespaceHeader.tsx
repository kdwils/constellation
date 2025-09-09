
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
            className="w-full p-4 bg-gray-50 hover:bg-gray-100 border-b border-gray-200 text-left transition-colors duration-200"
        >
            <div className="flex items-center justify-between">
                <span className="text-lg font-semibold text-gray-700">
                    {name}
                </span>
                <div className="flex items-center space-x-3">
                    <span className="text-xs text-gray-500 bg-white px-3 py-1 rounded-md font-medium border border-gray-200">
                        {resourceCount} RESOURCES
                    </span>
                    <div className="text-gray-400 text-sm">
                        {isCollapsed ? '▶' : '▼'}
                    </div>
                </div>
            </div>
        </button>
    );
}


