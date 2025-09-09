import { LabelList } from "./LabelList";

interface MetadataItemProps {
    label: string;
    value: string | number | string[] | number[];
}

export function MetadataItem({ label, value }: MetadataItemProps) {
    const items = Array.isArray(value) ? value : [value];
    
    return (
        <div className="flex items-center gap-1">
            <span className="text-slate-500">{label}:</span>
            <LabelList items={items} />
        </div>
    );
}