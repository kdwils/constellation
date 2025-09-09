import { PhaseStatus } from "./PhaseStatus";

interface ResourceHeaderProps {
    name: string;
    type: string;
    dotColor: string;
    phase?: string;
}

export function ResourceHeader({ name, type, dotColor, phase }: ResourceHeaderProps) {
    return (
        <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
                <div className={`w-2 h-2 ${dotColor} rounded-full`}></div>
                <span className="text-xs font-semibold text-slate-500 uppercase tracking-wide">{type}</span>
                <span className="font-medium text-slate-800">{name}</span>
            </div>
            {phase && <PhaseStatus phase={phase} />}
        </div>
    );
}