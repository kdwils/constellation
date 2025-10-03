import { useState, useRef, useEffect } from 'react';

interface TooltipProps {
    content: React.ReactNode;
    children: React.ReactNode;
    position?: 'top' | 'bottom' | 'left' | 'right';
    delay?: number;
}

export function Tooltip({ content, children, position = 'top', delay = 200 }: TooltipProps) {
    const [isVisible, setIsVisible] = useState(false);
    const [showTooltip, setShowTooltip] = useState(false);
    const timeoutRef = useRef<NodeJS.Timeout | null>(null);

    const handleMouseEnter = () => {
        if (timeoutRef.current) {
            clearTimeout(timeoutRef.current);
        }
        timeoutRef.current = setTimeout(() => {
            setIsVisible(true);
            setTimeout(() => setShowTooltip(true), 10);
        }, delay);
    };

    const handleMouseLeave = () => {
        if (timeoutRef.current) {
            clearTimeout(timeoutRef.current);
        }
        setShowTooltip(false);
        setTimeout(() => setIsVisible(false), 150);
    };

    useEffect(() => {
        return () => {
            if (timeoutRef.current) {
                clearTimeout(timeoutRef.current);
            }
        };
    }, []);

    const getTooltipClasses = () => {
        const base = "absolute z-50 px-3 py-2 text-xs text-white bg-gray-900 rounded-lg shadow-xl border border-gray-700 transition-opacity duration-150 pointer-events-none";
        const visible = showTooltip ? "opacity-100" : "opacity-0";
        
        switch (position) {
            case 'top':
                return `${base} ${visible} bottom-full left-1/2 transform -translate-x-1/2 mb-2 whitespace-nowrap`;
            case 'bottom':
                return `${base} ${visible} top-full left-1/2 transform -translate-x-1/2 mt-2 whitespace-nowrap`;
            case 'left':
                return `${base} ${visible} right-full top-1/2 transform -translate-y-1/2 mr-2 max-w-xs`;
            case 'right':
                return `${base} ${visible} left-full top-1/2 transform -translate-y-1/2 ml-2 max-w-xs`;
            default:
                return `${base} ${visible} whitespace-nowrap`;
        }
    };

    const getArrowClasses = () => {
        const base = "absolute w-2 h-2 bg-gray-900 border-gray-700 transform rotate-45";
        
        switch (position) {
            case 'top':
                return `${base} top-full left-1/2 transform -translate-x-1/2 -translate-y-1/2 border-r border-b`;
            case 'bottom':
                return `${base} bottom-full left-1/2 transform -translate-x-1/2 translate-y-1/2 border-l border-t`;
            case 'left':
                return `${base} left-full top-1/2 transform -translate-y-1/2 -translate-x-1/2 border-t border-r`;
            case 'right':
                return `${base} right-full top-1/2 transform -translate-y-1/2 translate-x-1/2 border-b border-l`;
            default:
                return base;
        }
    };

    return (
        <div
            className="relative inline-block"
            onMouseEnter={handleMouseEnter}
            onMouseLeave={handleMouseLeave}
        >
            {children}
            {isVisible && (
                <div className={getTooltipClasses()}>
                    {content}
                    <div className={getArrowClasses()} />
                </div>
            )}
        </div>
    );
}