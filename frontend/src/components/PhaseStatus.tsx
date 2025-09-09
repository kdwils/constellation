interface PhaseStatusProps {
    phase?: string;
}

export function PhaseStatus({ phase }: PhaseStatusProps) {
    if (!phase) return null;

    const getStatusConfig = (phase: string) => {
        switch (phase.toLowerCase()) {
            case 'running':
                return {
                    text: 'Running',
                    indicator: <div className="w-2 h-2 bg-green-500 rounded-full"></div>,
                };
            case 'pending':
            case 'containercreating':
            case 'init':
                return {
                    text: phase,
                    indicator: (
                        <div className="w-2 h-2 border border-yellow-500 border-t-transparent rounded-full animate-spin"></div>
                    ),
                };
            case 'failed':
            case 'crashloopbackoff':
            case 'error':
                return {
                    text: phase,
                    indicator: <div className="w-2 h-2 bg-red-500 rounded-full"></div>,
                };
            case 'succeeded':
                return {
                    text: 'Succeeded',
                    indicator: <div className="w-2 h-2 bg-green-500 rounded-full"></div>,
                };
            default:
                return {
                    text: phase,
                    indicator: <div className="w-2 h-2 bg-gray-400 rounded-full"></div>,
                };
        }
    };

    const { text, indicator } = getStatusConfig(phase);

    return (
        <div className="flex items-center space-x-1">
            {indicator}
            <span className="text-xs font-medium text-slate-600">{text}</span>
        </div>
    );
}