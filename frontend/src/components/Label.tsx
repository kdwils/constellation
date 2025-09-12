interface LabelProps {
    labelKey: string;
    value: string;
    highlighted?: boolean;
}

export function Label({ labelKey, value, highlighted = false }: LabelProps) {
    const baseClasses = "inline-block px-2 py-1 text-xs font-medium rounded break-words";
    const styleClasses = highlighted 
        ? "bg-amber-100 border border-amber-300"
        : "bg-gray-50 border border-gray-300";
    
    return (
        <span className={`${baseClasses} ${styleClasses}`}>
            <span className="text-slate-500">{labelKey}</span>
            <span className="text-slate-400">=</span>
            <span className="text-slate-700">{value}</span>
        </span>
    );
}