interface HttpRouteProps {
    name: string;
}

export function HttpRouteBox({ name }: HttpRouteProps) {
    return (
        <div className="ml-4 p-3 bg-purple-50 border-l-4 border-purple-400 rounded-r-lg shadow-sm">
            <div className="flex items-center space-x-2">
                <span className="text-purple-600">🛣️</span>
                <span className="font-medium text-purple-800">httproute:</span>
                <span className="text-purple-900">{name}</span>
            </div>
        </div>
    );
}
