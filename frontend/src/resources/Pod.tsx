interface PodBoxProps {
    name: string;
    labels?: Record<string, string>;
    ports?: number[];
    serviceSelectors?: Record<string, string>;
}

export function PodBox({ name, labels, ports, serviceSelectors }: PodBoxProps) {
    const hasMetadata = (labels && Object.keys(labels).length > 0) || (ports && ports.length > 0);
    
    const isLabelSelectedByService = (key: string, value: string): boolean => {
        return serviceSelectors ? serviceSelectors[key] === value : false;
    };

    return (
        <div className="ml-12 p-3 bg-white border-l-4 border-cyan-500 rounded-r-md shadow-sm">
            <div className="flex items-center space-x-3">
                <div className="w-2 h-2 bg-cyan-500 rounded-full"></div>
                <span className="text-xs font-semibold text-slate-500 uppercase tracking-wide">POD</span>
                <span className="font-medium text-slate-800">{name}</span>
            </div>
            {hasMetadata && (
                <div className="mt-2 ml-5 space-y-1 text-xs">
                    {labels && Object.keys(labels).length > 0 && (
                        <div className="flex items-start space-x-1">
                            <span className="text-slate-500 mt-0.5">🏷️</span>
                            <div className="flex-1 text-slate-600 font-medium">
                                <div className="flex flex-wrap gap-x-2 gap-y-1">
                                    {Object.entries(labels).map(([key, value]) => {
                                        const isSelected = isLabelSelectedByService(key, value);
                                        return (
                                            <span 
                                                key={key} 
                                                className={`break-words ${isSelected ? 'px-2 py-0.5 bg-amber-100 border border-amber-300 rounded' : ''}`}
                                            >
                                                <span className="text-slate-500">{key}</span>
                                                <span className="text-slate-400">=</span>
                                                <span className="text-slate-700">{value}</span>
                                            </span>
                                        );
                                    })}
                                </div>
                            </div>
                        </div>
                    )}
                    {ports && ports.length > 0 && (
                        <div className="flex items-center space-x-1">
                            <span className="text-slate-500">🔌</span>
                            <span className="text-slate-600 font-medium">{ports.join(', ')}</span>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}
