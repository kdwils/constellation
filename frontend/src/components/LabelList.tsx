import { LabelBadge } from "./LabelBadge";

interface LabelListProps<T extends number | string> {
    items: T[];
    formatter?: (item: T) => string;
}

export function LabelList<T extends number | string>({
    items,
    formatter = (item) => String(item)
}: LabelListProps<T>) {
    return (
        <div className="flex flex-wrap gap-1">
            {items.map((item, index) => (
                <LabelBadge key={`${item}-${index}`}>
                    {formatter(item)}
                </LabelBadge>
            ))}
        </div>
    );
}