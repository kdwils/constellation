interface HttpRouteProps {
    name: string;
    hostnames?: string[];
}

export function HttpRouteBox({ name, hostnames }: HttpRouteProps) {
    return (
        <div className="ml-4 p-3 bg-white border-l-4 border-violet-500 rounded-r-md shadow-sm">
            <div className="flex items-center space-x-3">
                <div className="w-2 h-2 bg-violet-500 rounded-full"></div>
                <span className="text-xs font-semibold text-slate-500 uppercase tracking-wide">HTTP ROUTE</span>
                <span className="font-medium text-slate-800">{name}</span>
            </div>
            {hostnames && hostnames.length > 0 && (
                <div className="mt-2 ml-5 flex items-center space-x-2 text-xs">
                    <span className="text-slate-500">🌐</span>
                    <span className="text-slate-600 font-medium">{hostnames.join(', ')}</span>
                </div>
            )}
        </div>
    );
}
