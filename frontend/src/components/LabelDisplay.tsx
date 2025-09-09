interface LabelDisplayProps {
    labels: Record<string, string>;
    highlightedLabels?: Record<string, string>;
    alwaysHighlight?: boolean;
}

export function LabelDisplay({ labels, highlightedLabels, alwaysHighlight = false }: LabelDisplayProps) {
    const isLabelHighlighted = (key: string, value: string): boolean => {
        if (alwaysHighlight) return true;
        return highlightedLabels ? highlightedLabels[key] === value : false;
    };

    return (
        <div className="flex flex-wrap gap-x-2 gap-y-1">
            {Object.entries(labels).map(([key, value]) => {
                const isHighlighted = isLabelHighlighted(key, value);
                return (
                    <span 
                        key={key} 
                        className={`break-words ${isHighlighted ? 'px-2 py-0.5 bg-amber-100 border border-amber-300 rounded' : ''}`}
                    >
                        <span className="text-slate-500">{key}</span>
                        <span className="text-slate-400">=</span>
                        <span className="text-slate-700">{value}</span>
                    </span>
                );
            })}
        </div>
    );
}