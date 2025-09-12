import { calculateHealthPercentage } from "../../utils/resourceStats";

interface HealthIndicatorProps {
    healthyPods: number;
    totalPods: number;
    variant?: 'card' | 'bar' | 'icon';
    size?: 'sm' | 'md' | 'lg';
}

export function HealthIndicator({ 
    healthyPods, 
    totalPods, 
    variant = 'card', 
    size = 'md' 
}: HealthIndicatorProps) {
    const healthPercent = calculateHealthPercentage(healthyPods, totalPods);
    
    const getHealthColor = (percent: number) => {
        if (percent === 100) return 'green';
        if (percent > 50) return 'yellow';
        return 'red';
    };
    
    const color = getHealthColor(healthPercent);
    
    if (variant === 'card') {
        const sizeClasses = {
            sm: 'p-3',
            md: 'p-4',
            lg: 'p-6'
        };
        
        return (
            <div className={`bg-white rounded-lg border border-gray-200 ${sizeClasses[size]}`}>
                <div className="flex items-center">
                    <div className={`p-3 rounded-full ${
                        color === 'green' ? 'bg-green-100' :
                        color === 'yellow' ? 'bg-yellow-100' : 'bg-red-100'
                    }`}>
                        <div className={`w-6 h-6 ${
                            color === 'green' ? 'text-green-600' :
                            color === 'yellow' ? 'text-yellow-600' : 'text-red-600'
                        }`}>
                            {color === 'green' ? '✅' : color === 'yellow' ? '⚠️' : '❌'}
                        </div>
                    </div>
                    <div className="ml-4">
                        <p className="text-sm font-medium text-gray-600">
                            {variant === 'card' ? 'Pod Health' : 'Running Pods'}
                        </p>
                        <p className="text-2xl font-semibold text-gray-900">
                            {variant === 'card' ? `${healthPercent}%` : `${healthyPods}/${totalPods}`}
                        </p>
                    </div>
                </div>
            </div>
        );
    }
    
    if (variant === 'bar') {
        return (
            <div className="flex items-center space-x-2">
                <span className="text-xs text-gray-600">Pod Health:</span>
                <div className="flex-1 max-w-xs">
                    <div className={`h-2 rounded-full ${
                        color === 'green' ? 'bg-green-200' :
                        color === 'yellow' ? 'bg-yellow-200' : 'bg-red-200'
                    }`}>
                        <div
                            className={`h-full rounded-full transition-all duration-300 ${
                                color === 'green' ? 'bg-green-500' :
                                color === 'yellow' ? 'bg-yellow-500' : 'bg-red-500'
                            }`}
                            style={{ width: `${healthPercent}%` }}
                        />
                    </div>
                </div>
                <span className="text-xs text-gray-600">{healthPercent}%</span>
            </div>
        );
    }
    
    if (variant === 'icon') {
        return (
            <div className="text-center">
                <div className={`text-sm ${
                    color === 'green' ? 'text-green-600' :
                    color === 'yellow' ? 'text-yellow-600' : 'text-red-600'
                }`}>
                    {color === 'green' ? '✅' : color === 'yellow' ? '⚠️' : '❌'}
                </div>
                <div className="text-xs text-gray-500 uppercase tracking-wide">
                    {healthPercent}%
                </div>
            </div>
        );
    }
    
    return null;
}