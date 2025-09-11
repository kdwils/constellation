interface MetadataLabelProps {
    children: string;
}

export function MetadataLabel({ children }: MetadataLabelProps) {
    return <span className="text-slate-500">{children}:</span>;
}