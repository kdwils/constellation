import { LabelDisplay } from "./LabelDisplay";
import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";
import { MetadataItem } from "./MetadataItem";
import { MetadataContainer } from "./MetadataContainer";
import { CompactMetadataRow } from "./CompactMetadataRow";
import { MetadataLabel } from "./MetadataLabel";
import { PortMappingList } from "./PortMappingList";
import { Tooltip } from "./Tooltip";
import type { ContainerPortInfo, ServiceHealthInfo, HealthCheckEntry } from "../types";

interface ServiceBoxProps {
    name: string;
    selectors?: Record<string, string>;
    portMappings?: string[];
    isTargetedByRoute?: boolean;
    serviceType?: string;
    clusterIps?: string[];
    externalIps?: string[];
    childContainerPorts?: ContainerPortInfo[];
    healthInfo?: ServiceHealthInfo;
}

export function ServiceBox({ name, selectors, portMappings, isTargetedByRoute, serviceType, clusterIps, externalIps, childContainerPorts, healthInfo }: ServiceBoxProps) {
    const hasMetadata = (selectors && Object.keys(selectors).length > 0) || (portMappings && portMappings.length > 0) || serviceType || (clusterIps && clusterIps.length > 0) || (externalIps && externalIps.length > 0) || healthInfo;

    const getHealthIndicator = () => {
        if (!healthInfo) return null;
        
        const { status, uptime, last_check } = healthInfo;
        const lastCheckDate = new Date(last_check);
        const isRecent = Date.now() - lastCheckDate.getTime() < 5 * 60 * 1000; // 5 minutes
        
        const statusColor = status === 'healthy' ? 'text-green-600' : 
                           status === 'unhealthy' ? 'text-red-600' : 'text-gray-600';
        const statusIcon = status === 'healthy' ? '●' : 
                          status === 'unhealthy' ? '●' : '●';
        
        return (
            <div className="flex items-center gap-2">
                <span className={`${statusColor} text-xs`} title={`Status: ${status}`}>
                    {statusIcon}
                </span>
                <span className="text-xs text-gray-600">
                    {uptime.toFixed(1)}% uptime
                </span>
                {!isRecent && (
                    <span className="text-xs text-orange-500" title="Health check data is stale">
                        ⚠
                    </span>
                )}
            </div>
        );
    };

    return (
        <ResourceBox borderColor={isTargetedByRoute ? "border-amber-400" : "border-amber-300"} marginLeft="ml-0">
            <ResourceHeader name={name} type="SERVICE" dotColor="bg-amber-500" />
            {hasMetadata && (
                <MetadataContainer>
                    <CompactMetadataRow>
                        {serviceType && <MetadataItem label="type" value={serviceType} />}
                        {portMappings && portMappings.length > 0 && (
                            <div className="flex items-center gap-1">
                                <MetadataLabel>ports</MetadataLabel>
                                <PortMappingList portMappings={portMappings} childContainerPorts={childContainerPorts} />
                            </div>
                        )}
                        {clusterIps && clusterIps.length > 0 && <MetadataItem label="cluster-ips" value={clusterIps} />}
                        {externalIps && externalIps.length > 0 && <MetadataItem label="external-ips" value={externalIps} />}
                        {healthInfo && (
                            <div className="flex items-center gap-1">
                                <MetadataLabel>health</MetadataLabel>
                                {getHealthIndicator()}
                            </div>
                        )}
                    </CompactMetadataRow>

                    {selectors && Object.keys(selectors).length > 0 && (
                        <MetadataRow icon="selectors" alignItems="start">
                            <div className="flex-1 text-slate-600 font-medium">
                                <LabelDisplay labels={selectors} alwaysHighlight={true} />
                            </div>
                        </MetadataRow>
                    )}
                </MetadataContainer>
            )}
        </ResourceBox>
    );
}
