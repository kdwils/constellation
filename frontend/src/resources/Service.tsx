interface ServiceBoxProps {
    name: string;
}

export function ServiceBox({ name }: ServiceBoxProps) {
    return (
        <div className="ml-8 p-3 bg-orange-50 border-l-4 border-orange-400 rounded-r-lg shadow-sm">
            <div className="flex items-center space-x-2">
                <span className="text-orange-600">⚙️</span>
                <span className="font-medium text-orange-800">service:</span>
                <span className="text-orange-900">{name}</span>
            </div>
        </div>
    );
}
