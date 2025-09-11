import { LabelBadge } from "./LabelBadge";

interface HighlightableListProps<T> {
    items: T[];
    getDisplayText: (item: T, index: number) => string;
    isHighlighted: (item: T, index: number) => boolean;
    getKey?: (item: T, index: number) => string;
}

export function HighlightableList<T>({ 
    items, 
    getDisplayText, 
    isHighlighted,
    getKey = (_, index) => index.toString()
}: HighlightableListProps<T>) {
    return (
        <div className="flex flex-wrap gap-x-2 gap-y-1">
            {items.map((item, index) => (
                <LabelBadge key={getKey(item, index)} highlighted={isHighlighted(item, index)}>
                    {getDisplayText(item, index)}
                </LabelBadge>
            ))}
        </div>
    );
}