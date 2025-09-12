interface SidebarItemProps {
    name: string;
    isSelected: boolean;
    onClick: () => void;
    stats: {
        pods?: number;
        healthyPods?: number;
        totalResources?: number;
        namespaces?: number;
    };
}

export function SidebarItem({ name, isSelected, onClick, stats }: SidebarItemProps) {
    const healthPercent = stats.pods && stats.healthyPods
        ? Math.round((stats.healthyPods / stats.pods) * 100)
        : 0;

    return (
        <button
            onClick={onClick}
            className={`w-full p-4 rounded-lg text-left mb-2 transition-all duration-200 ${isSelected
                ? 'bg-blue-50 border-2 border-blue-200 shadow-sm'
                : 'bg-gray-50 hover:bg-gray-100 border-2 border-transparent'
                }`}
        >
            <div className="flex items-center gap-3">
                <div className="flex-1 min-w-0">
                    <h3 className={`font-medium truncate ${isSelected ? 'text-blue-900' : 'text-gray-900'}`}>
                        {name}
                    </h3>
                </div>


                <div className="flex items-center space-x-3 flex-shrink-0">
                    {stats.totalResources !== undefined && (
                        <div className="text-right">
                            <div className="text-xs font-medium text-gray-900">{stats.totalResources}</div>
                            <div className="text-xs text-gray-500">resources</div>
                        </div>
                    )}
                    {stats.namespaces !== undefined && (
                        <div className="text-right">
                            <div className="text-xs font-medium text-gray-900">{stats.namespaces}</div>
                            <div className="text-xs text-gray-500">namespaces</div>
                        </div>
                    )}
                    {stats.pods !== undefined && stats.pods > 0 && (
                        <div className="flex flex-col items-center gap-1">
                            <div className="text-center">
                                <div className="text-xs font-medium text-gray-900">{stats.healthyPods}/{stats.pods} <span className="text-gray-500">pods</span></div>
                            </div>
                            <div className={`w-16 h-2 rounded-full ${healthPercent === 100 ? 'bg-green-200' :
                                healthPercent > 50 ? 'bg-yellow-200' : 'bg-red-200'
                                }`}>
                                <div
                                    className={`h-full rounded-full transition-all duration-300 ${healthPercent === 100 ? 'bg-green-500' :
                                        healthPercent > 50 ? 'bg-yellow-500' : 'bg-red-500'
                                        }`}
                                    style={{ width: `${healthPercent}%` }}
                                />
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </button>
    );
}