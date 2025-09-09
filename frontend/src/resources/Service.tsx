interface ServiceBoxProps {
    name: string;
    selectors?: Record<string, string>;
    ports?: number[];
}

export function ServiceBox({ name, selectors, ports }: ServiceBoxProps) {
    const hasMetadata = (selectors && Object.keys(selectors).length > 0) || (ports && ports.length > 0);
    
    return (
        <div className="ml-8 p-3 bg-white border-l-4 border-amber-500 rounded-r-md shadow-sm">
            <div className="flex items-center space-x-3">
                <div className="w-2 h-2 bg-amber-500 rounded-full"></div>
                <span className="text-xs font-semibold text-slate-500 uppercase tracking-wide">SERVICE</span>
                <span className="font-medium text-slate-800">{name}</span>
            </div>
            {hasMetadata && (
                <div className="mt-2 ml-5 space-y-1 text-xs">
                    {selectors && Object.keys(selectors).length > 0 && (
                        <div className="flex items-start space-x-1">
                            <span className="text-slate-500 mt-0.5">🎯</span>
                            <div className="flex-1 text-slate-600 font-medium">
                                <div className="flex flex-wrap gap-x-2 gap-y-1">
                                    {Object.entries(selectors).map(([key, value]) => (
                                        <span key={key} className="break-words">
                                            <span className="text-slate-500">{key}</span>
                                            <span className="text-slate-400">=</span>
                                            <span className="text-slate-700">{value}</span>
                                        </span>
                                    ))}
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
