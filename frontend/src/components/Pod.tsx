import { LabelDisplay } from "./LabelDisplay";
import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";
import { MetadataItem } from "./MetadataItem";
import { MetadataContainer } from "./MetadataContainer";
import { CompactMetadataRow } from "./CompactMetadataRow";
import { ContainerPortList } from "./ContainerPortList";
import type { ContainerPortInfo } from "../ResourceNode";

interface PodBoxProps {
    name: string;
    labels?: Record<string, string>;
    containerPorts?: ContainerPortInfo[];
    serviceSelectors?: Record<string, string>;
    targetPorts?: number[];
    targetPortNames?: string[];
    phase?: string;
    podIp?: string;
}

export function PodBox({ name, labels, containerPorts, serviceSelectors, targetPorts, targetPortNames, phase, podIp }: PodBoxProps) {
    const hasMetadata = (labels && Object.keys(labels).length > 0) || (containerPorts && containerPorts.length > 0) || podIp;

    return (
        <ResourceBox borderColor="border-cyan-500" marginLeft="ml-12">
            <ResourceHeader name={name} type="POD" dotColor="bg-cyan-500" phase={phase} />
            {hasMetadata && (
                <MetadataContainer>
                    <CompactMetadataRow>
                        {containerPorts && containerPorts.length > 0 && (
                            <div className="flex items-center gap-1">
                                <span className="text-slate-500">ports:</span>
                                <ContainerPortList containerPorts={containerPorts} highlightedPorts={targetPorts} highlightedPortNames={targetPortNames} />
                            </div>
                        )}
                        {podIp && <MetadataItem label="ip" value={podIp} />}
                    </CompactMetadataRow>
                    
                    {labels && Object.keys(labels).length > 0 && (
                        <MetadataRow icon="labels" alignItems="start">
                            <div className="flex-1 text-slate-600 font-medium">
                                <LabelDisplay labels={labels} highlightedLabels={serviceSelectors} />
                            </div>
                        </MetadataRow>
                    )}
                </MetadataContainer>
            )}
        </ResourceBox>
    );
}
