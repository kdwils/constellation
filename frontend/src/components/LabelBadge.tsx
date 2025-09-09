interface LabelBadgeProps {
    children: React.ReactNode;
    highlighted?: boolean;
    className?: string;
}

export function LabelBadge({ children, highlighted = false, className = "" }: LabelBadgeProps) {
    const baseClasses = "inline-block px-2 py-1 text-xs font-medium rounded break-words";
    const styleClasses = highlighted 
        ? "bg-amber-100 border border-amber-300"
        : "text-slate-700 bg-gray-100 border border-gray-300";
    
    return (
        <span className={`${baseClasses} ${styleClasses} ${className}`}>
            {children}
        </span>
    );
}