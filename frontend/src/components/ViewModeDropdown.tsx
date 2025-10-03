import { useState } from "react";

interface ViewModeDropdownProps {
    value: 'health' | 'namespace' | 'group';
    onChange: (value: 'health' | 'namespace' | 'group') => void;
}

const options = [
    { value: 'health' as const, label: '❤️ Health Dashboard' },
    { value: 'namespace' as const, label: '📁 Namespaces' },
    { value: 'group' as const, label: '🏷️ Groups' }
];

export function ViewModeDropdown({ value, onChange }: ViewModeDropdownProps) {
    const [isOpen, setIsOpen] = useState(false);
    
    const selectedOption = options.find(option => option.value === value);
    
    return (
        <div className="relative">
            <button
                onClick={() => setIsOpen(!isOpen)}
                className="w-full px-3 py-2 text-sm font-medium bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 flex items-center justify-between"
            >
                <span>{selectedOption?.label}</span>
                <span className={`transform transition-transform duration-200 ${isOpen ? 'rotate-90' : ''}`}>
                    ›
                </span>
            </button>
            
            {isOpen && (
                <div className="absolute top-full left-0 right-0 mt-1 bg-white border border-gray-300 rounded-md shadow-lg z-10">
                    {options.map((option) => (
                        <button
                            key={option.value}
                            onClick={() => {
                                onChange(option.value);
                                setIsOpen(false);
                            }}
                            className={`w-full px-3 py-2 text-sm text-left hover:bg-gray-50 first:rounded-t-md last:rounded-b-md ${
                                option.value === value ? 'bg-blue-50 text-blue-600' : 'text-gray-700'
                            }`}
                        >
                            {option.label}
                        </button>
                    ))}
                </div>
            )}
            
            {isOpen && (
                <div 
                    className="fixed inset-0 z-0" 
                    onClick={() => setIsOpen(false)}
                />
            )}
        </div>
    );
}