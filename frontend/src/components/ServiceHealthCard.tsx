import type { ResourceNode } from "../types";

interface ServiceHealthCardProps {
    data: ResourceNode[];
}

export function ServiceHealthCard({ data }: ServiceHealthCardProps) {
    // Extract all services with health info across all namespaces
    const servicesWithHealth = collectServicesWithHealth(data);

    const totalServices = servicesWithHealth.length;
    const healthyServices = servicesWithHealth.filter(s => s.health_info?.status === 'healthy').length;
    const averageUptime = totalServices > 0
        ? servicesWithHealth.reduce((sum, s) => sum + (s.health_info?.uptime || 0), 0) / totalServices
        : 0;

    const getHealthColor = (healthy: number, total: number) => {
        if (total === 0) return 'gray';
        const percentage = (healthy / total) * 100;
        if (percentage === 100) return 'green';
        if (percentage > 75) return 'yellow';
        return 'red';
    };

    const healthColor = getHealthColor(healthyServices, totalServices);

    return (
        <div className="bg-white rounded-lg border border-gray-200 p-4">
            <div className="flex items-center">
                <div className={`p-3 rounded-full ${healthColor === 'green' ? 'bg-green-100' :
                        healthColor === 'yellow' ? 'bg-yellow-100' :
                            healthColor === 'red' ? 'bg-red-100' : 'bg-gray-100'
                    }`}>
                    <div className={`w-6 h-6 flex items-center justify-center ${healthColor === 'green' ? 'text-green-600' :
                            healthColor === 'yellow' ? 'text-yellow-600' :
                                healthColor === 'red' ? 'text-red-600' : 'text-gray-600'
                        }`}>
                        {healthColor === 'green' ? '✅' :
                            healthColor === 'yellow' ? '⚠️' :
                                healthColor === 'red' ? '❌' : '➖'}
                    </div>
                </div>
                <div className="ml-4">
                    <p className="text-sm font-medium text-gray-600">Service Health</p>
                    <div className="flex items-center space-x-2">
                        <p className="text-xl font-semibold text-gray-900">
                            {healthyServices}/{totalServices}
                        </p>
                        {totalServices > 0 && (
                            <span className="text-sm text-gray-500">
                                ({averageUptime.toFixed(1)}% avg uptime)
                            </span>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}

function collectServicesWithHealth(nodes: ResourceNode[]): ResourceNode[] {
    const services: ResourceNode[] = [];

    function traverse(node: ResourceNode) {
        if (node.kind === 'Service' && node.health_info) {
            services.push(node);
        }

        if (node.relatives) {
            node.relatives.forEach(traverse);
        }
    }

    nodes.forEach(traverse);
    return services;
}