import { LabelDisplay } from "./LabelDisplay";
import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";
import { MetadataItem } from "./MetadataItem";
import { MetadataContainer } from "./MetadataContainer";
import { CompactMetadataRow } from "./CompactMetadataRow";
import { MetadataLabel } from "./MetadataLabel";
import { PortMappingList } from "./PortMappingList";
import type { ContainerPortInfo } from "../ResourceNode";

interface ServiceBoxProps {
    name: string;
    selectors?: Record<string, string>;
    portMappings?: string[];
    isTargetedByRoute?: boolean;
    serviceType?: string;
    clusterIps?: string[];
    externalIps?: string[];
    childContainerPorts?: ContainerPortInfo[];
}

export function ServiceBox({ name, selectors, portMappings, isTargetedByRoute, serviceType, clusterIps, externalIps, childContainerPorts }: ServiceBoxProps) {
    const hasMetadata = (selectors && Object.keys(selectors).length > 0) || (portMappings && portMappings.length > 0) || serviceType || (clusterIps && clusterIps.length > 0) || (externalIps && externalIps.length > 0);

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
