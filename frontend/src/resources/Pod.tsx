interface PodBoxProps {
    name: string;
}

export function PodBox({ name }: PodBoxProps) {
    return (
        <div className="ml-12 p-3 bg-teal-50 border-l-4 border-teal-400 rounded-r-lg shadow-sm">
            <div className="flex items-center space-x-2">
                <span className="text-teal-600">📦</span>
                <span className="font-medium text-teal-800">pod:</span>
                <span className="text-teal-900">{name}</span>
            </div>
        </div>
    );
}
