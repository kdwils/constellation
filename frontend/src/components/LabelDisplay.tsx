import { Label } from "./Label";

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
            {Object.entries(labels).map(([key, value]) => (
                <Label
                    key={key}
                    labelKey={key}
                    value={value}
                    highlighted={isLabelHighlighted(key, value)}
                />
            ))}
        </div>
    );
}