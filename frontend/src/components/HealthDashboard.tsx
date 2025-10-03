import { useState } from "react";
import type { ResourceNode, ServiceHealthInfo, HealthCheckEntry } from "../types";
import { Tooltip } from "./Tooltip";

interface HealthDashboardProps {
    data: ResourceNode[];
}

interface ServiceHealthRow {
    serviceName: string;
    namespace: string;
    healthInfo: ServiceHealthInfo;
    uptime24h: number;
    uptime30d: number;
    avgResponse: number;
    currentResponse: number;
}

export function HealthDashboard({ data }: HealthDashboardProps) {
    const [searchTerm, setSearchTerm] = useState("");
    const [filterBy, setFilterBy] = useState<"Nothing" | "Healthy" | "Unhealthy">("Nothing");
    const [sortBy, setSortBy] = useState<"Name" | "Status" | "Uptime">("Name");
    
    // Extract all services with health info
    const healthRows = extractHealthRows(data);
    
    // Filter and sort services
    const filteredAndSortedRows = healthRows
        .filter(row => {
            const matchesSearch = row.serviceName.toLowerCase().includes(searchTerm.toLowerCase()) ||
                                row.namespace.toLowerCase().includes(searchTerm.toLowerCase());
            
            if (filterBy === "Nothing") return matchesSearch;
            if (filterBy === "Healthy") return matchesSearch && row.healthInfo.status === 'healthy';
            if (filterBy === "Unhealthy") return matchesSearch && row.healthInfo.status === 'unhealthy';
            
            return matchesSearch;
        })
        .sort((a, b) => {
            if (sortBy === "Name") return a.serviceName.localeCompare(b.serviceName);
            if (sortBy === "Status") return a.healthInfo.status.localeCompare(b.healthInfo.status);
            if (sortBy === "Uptime") return b.uptime24h - a.uptime24h;
            return 0;
        });

    // Show empty state if no services with health info
    if (healthRows.length === 0) {
        return (
            <div className="flex-1 flex items-center justify-center p-6">
                <div className="text-center">
                    <div className="text-6xl mb-4">🏥</div>
                    <h2 className="text-xl font-semibold text-gray-900 mb-2">No Health Checks Yet</h2>
                    <p className="text-gray-600 max-w-md mx-auto">
                        Health checks will appear here once the health checker discovers services with readiness or liveness probes.
                    </p>
                </div>
            </div>
        );
    }

    return (
        <div className="flex-1 p-6">
            {/* Header */}
            <div className="mb-6">
                <h1 className="text-2xl font-bold text-gray-900 mb-2">Service Health Dashboard</h1>
                <p className="text-gray-600">Monitor the health of your in-cluster services</p>
            </div>
            
            {/* Search and Filters */}
            <div className="flex items-center space-x-4 mb-6">
                <div className="flex-1 relative">
                    <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                        <svg className="h-5 w-5 text-gray-400" viewBox="0 0 20 20" fill="currentColor">
                            <path fillRule="evenodd" d="M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z" clipRule="evenodd" />
                        </svg>
                    </div>
                    <input
                        type="text"
                        placeholder="Search services..."
                        className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                        value={searchTerm}
                        onChange={(e) => setSearchTerm(e.target.value)}
                    />
                </div>
                
                <div className="flex items-center space-x-2">
                    <span className="text-gray-600 text-sm">Filter by:</span>
                    <select
                        className="border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        value={filterBy}
                        onChange={(e) => setFilterBy(e.target.value as typeof filterBy)}
                    >
                        <option value="Nothing">Nothing</option>
                        <option value="Healthy">Healthy</option>
                        <option value="Unhealthy">Unhealthy</option>
                    </select>
                </div>
                
                <div className="flex items-center space-x-2">
                    <span className="text-gray-600 text-sm">Sort by:</span>
                    <select
                        className="border border-gray-300 rounded-lg px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        value={sortBy}
                        onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
                    >
                        <option value="Name">Name</option>
                        <option value="Status">Status</option>
                        <option value="Uptime">Uptime</option>
                    </select>
                </div>
            </div>

            {/* Service Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {filteredAndSortedRows.map((row) => (
                    <ServiceHealthCardGrid
                        key={`${row.namespace}/${row.serviceName}`}
                        service={row}
                    />
                ))}
            </div>
        </div>
    );
}

interface ServiceHealthCardGridProps {
    service: ServiceHealthRow;
}

function ServiceHealthCardGrid({ service }: ServiceHealthCardGridProps) {
    const { healthInfo } = service;
    const isHealthy = healthInfo.status === 'healthy';
    const isUnhealthy = healthInfo.status === 'unhealthy';
    
    const getTimeSinceLastCheck = () => {
        if (healthInfo.history.length === 0) return 'No checks yet';
        
        const lastCheck = new Date(healthInfo.history[healthInfo.history.length - 1].timestamp);
        const now = new Date();
        const diffMs = now.getTime() - lastCheck.getTime();
        
        const diffSeconds = Math.floor(diffMs / 1000);
        const diffMinutes = Math.floor(diffSeconds / 60);
        const diffHours = Math.floor(diffMinutes / 60);
        
        if (diffHours > 0) return `~${diffHours}h ago`;
        if (diffMinutes > 0) return `~${diffMinutes}m ago`;
        return `~${diffSeconds}s ago`;
    };
    
    const getResponseTime = () => {
        if (healthInfo.history.length === 0) return 'No data';
        
        const lastEntry = healthInfo.history[healthInfo.history.length - 1];
        const responseTimeMs = Math.round(lastEntry.latency / 1000000);
        
        return `~${responseTimeMs}ms`;
    };

    return (
        <div className="bg-white rounded-lg border border-gray-200 p-4 shadow-sm">
            {/* Header */}
            <div className="flex items-center justify-between mb-4">
                <div>
                    <h3 className="text-lg font-medium text-gray-900">{service.serviceName}</h3>
                    <div className="flex items-center space-x-2 text-sm text-gray-500">
                        <span>{service.namespace}</span>
                        <span>•</span>
                        <span className="truncate max-w-40" title={healthInfo.url}>{healthInfo.url}</span>
                    </div>
                </div>
                <div className={`px-3 py-1 rounded-full text-sm font-medium ${
                    isHealthy ? 'bg-green-500 text-white' :
                    isUnhealthy ? 'bg-red-500 text-white' : 
                    'bg-gray-500 text-white'
                }`}>
                    {isHealthy ? 'Healthy' : isUnhealthy ? 'Unhealthy' : 'Unknown'}
                </div>
            </div>

            {/* Response Time Info */}
            <div className="flex justify-between items-center text-sm text-gray-500 mb-4">
                <span>{getTimeSinceLastCheck()}</span>
                <span className="text-right">{getResponseTime()}</span>
            </div>

            {/* Health History Bar */}
            <div className="mb-4">
                <HealthHistoryBar 
                    history={healthInfo.history}
                />
            </div>

            {/* Time Range Labels */}
            <div className="flex justify-between text-xs text-gray-400">
                <span>8 hours ago</span>
                <span>1 minute ago</span>
            </div>
        </div>
    );
}


interface HealthHistoryBarProps {
    history: HealthCheckEntry[];
    showLabels?: boolean;
}

function HealthHistoryBar({ history, showLabels = false }: HealthHistoryBarProps) {
    const minOvals = 15; // Always show 15 ovals
    const recentHistory = history.slice(-minOvals);
    
    const formatDuration = (latency: number) => {
        const ms = Math.round(latency / 1000000);
        if (ms < 1000) return `${ms}ms`;
        return `${(ms / 1000).toFixed(2)}s`;
    };

    const formatTimestamp = (timestamp: string) => {
        const date = new Date(timestamp);
        return date.toLocaleString();
    };

    const createTooltipContent = (entry: HealthCheckEntry) => (
        <div className="text-left space-y-1">
            <div className="font-medium">Health Check Details</div>
            <div><span className="font-medium">URL:</span> {entry.url}</div>
            <div><span className="font-medium">Method:</span> {entry.method}</div>
            <div><span className="font-medium">Status:</span> {entry.status}</div>
            {entry.response_code && (
                <div><span className="font-medium">Response Code:</span> {entry.response_code}</div>
            )}
            <div><span className="font-medium">Response Time:</span> {formatDuration(entry.latency)}</div>
            <div><span className="font-medium">Timestamp:</span> {formatTimestamp(entry.timestamp)}</div>
            {entry.error && (
                <div><span className="font-medium">Error:</span> {entry.error}</div>
            )}
        </div>
    );

    // Create array with real data + gray placeholders to always have minOvals count
    const ovals = Array.from({ length: minOvals }, (_, index) => {
        const entry = recentHistory[index];
        const isGray = !entry;
        
        const color = isGray ? 'bg-gray-300' :
                     entry.status === 'healthy' ? 'bg-green-500' :
                     entry.status === 'unhealthy' ? 'bg-red-500' : 
                     'bg-gray-400';
        
        const oval = (
            <div
                key={index}
                className={`${color} w-3 h-3 rounded-full ${
                    isGray ? '' : 'cursor-help hover:scale-110 transition-transform duration-150'
                }`}
            />
        );

        // Only add tooltip for non-gray ovals
        if (isGray) {
            return oval;
        }

        return (
            <Tooltip
                key={index}
                content={createTooltipContent(entry)}
                position="top"
                delay={100}
            >
                {oval}
            </Tooltip>
        );
    });

    return (
        <div className="space-y-2">
            <div className="flex gap-1 justify-center">
                {ovals}
            </div>
            {showLabels && (
                <div className="flex justify-between text-xs text-gray-400">
                    <span>Earlier</span>
                    <span>Recent</span>
                </div>
            )}
        </div>
    );
}

function extractHealthRows(data: ResourceNode[]): ServiceHealthRow[] {
    const services: ServiceHealthRow[] = [];
    
    function traverse(node: ResourceNode) {
        if (node.kind === 'Service' && node.health_info) {
            const healthInfo = node.health_info;
            const recentHistory = healthInfo.history.slice(-48); // Last 48 checks (24h if every 30min)
            const veryRecentHistory = healthInfo.history.slice(-720); // Last 720 checks (30d if every hour)
            
            const uptime24h = recentHistory.length > 0 
                ? (recentHistory.filter(h => h.status === 'healthy').length / recentHistory.length) * 100
                : 0;
                
            const uptime30d = veryRecentHistory.length > 0
                ? (veryRecentHistory.filter(h => h.status === 'healthy').length / veryRecentHistory.length) * 100
                : 0;
                
            const avgResponse = recentHistory.length > 0
                ? recentHistory.reduce((sum, h) => sum + (h.latency / 1000000), 0) / recentHistory.length
                : 0;
                
            const currentResponse = healthInfo.history.length > 0
                ? healthInfo.history[healthInfo.history.length - 1].latency / 1000000
                : 0;
            
            services.push({
                serviceName: node.name,
                namespace: node.namespace || 'default',
                healthInfo,
                uptime24h,
                uptime30d,
                avgResponse,
                currentResponse,
            });
        }
        
        if (node.relatives) {
            node.relatives.forEach(traverse);
        }
    }
    
    data.forEach(traverse);
    return services.sort((a, b) => a.serviceName.localeCompare(b.serviceName));
}

