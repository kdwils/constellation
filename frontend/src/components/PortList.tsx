interface PortListProps {
    ports: number[];
}

export function PortList({ ports }: PortListProps) {
    return (
        <div className="flex flex-wrap gap-1">
            {ports.map((port) => (
                <span
                    key={port}
                    className="inline-block px-2 py-1 text-xs font-medium text-slate-700 bg-gray-100 border border-gray-300 rounded"
                >
                    {port}
                </span>
            ))}
        </div>
    );
}