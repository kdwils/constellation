interface IngressProps {
    name: string;
}

export function IngressBox({ name }: IngressProps) {
    return (
        <div className="ml-4 p-3 bg-green-50 border-l-4 border-green-400 rounded-r-lg shadow-sm">
            <div className="flex items-center space-x-2">
                <span className="text-green-600">🌐</span>
                <span className="font-medium text-green-800">ingress:</span>
                <span className="text-green-900">{name}</span>
            </div>
        </div>
    );
}