import type { ReactNode } from "react";

interface CollapsibleSectionProps {
    title: string;
    isCollapsed: boolean;
    onToggle: () => void;
    children: ReactNode;
}

export function CollapsibleSection({ title, isCollapsed, onToggle, children }: CollapsibleSectionProps) {
    return (
        <div className="bg-white rounded-lg border border-gray-200 overflow-hidden">
            <button
                onClick={onToggle}
                className="w-full px-6 py-4 bg-gray-50 hover:bg-gray-100 border-b border-gray-200 text-left transition-colors duration-200 flex items-center justify-between"
            >
                <h2 className="text-xl font-semibold text-gray-900">{title}</h2>
                <span className={`transform transition-transform duration-200 ${isCollapsed ? '' : 'rotate-90'}`}>
                    ›
                </span>
            </button>

            <div className={`transition-all duration-300 ease-in-out overflow-hidden ${isCollapsed ? 'max-h-0 opacity-0' : 'max-h-[600px] opacity-100'
                }`}>
                <div className="p-6 overflow-y-auto max-h-[550px]">
                    {children}
                </div>
            </div>
        </div>
    );
}