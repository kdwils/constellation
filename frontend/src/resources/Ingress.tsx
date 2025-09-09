interface IngressProps {
    name: string;
}

export function IngressBox({ name }: IngressProps) {
    return (
        <div className="ml-4 p-3 bg-white border-l-4 border-emerald-500 rounded-r-md shadow-sm">
            <div className="flex items-center space-x-3">
                <div className="w-2 h-2 bg-emerald-500 rounded-full"></div>
                <span className="text-xs font-semibold text-slate-500 uppercase tracking-wide">INGRESS</span>
                <span className="font-medium text-slate-800">{name}</span>
            </div>
        </div>
    );
}