import { LabelDisplay } from "./LabelDisplay";
import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";
import { MetadataItem } from "./MetadataItem";
import { MetadataContainer } from "./MetadataContainer";
import { CompactMetadataRow } from "./CompactMetadataRow";
import { MetadataLabel } from "./MetadataLabel";
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
    podIps?: string[];
}

export function PodBox({ name, labels, containerPorts, serviceSelectors, targetPorts, targetPortNames, phase, podIps }: PodBoxProps) {
    const hasMetadata = (labels && Object.keys(labels).length > 0) || (containerPorts && containerPorts.length > 0) || (podIps && podIps.length > 0);

    return (
        <ResourceBox borderColor="border-cyan-300" marginLeft="ml-0">
            <ResourceHeader name={name} type="POD" dotColor="bg-cyan-500" phase={phase} />
            {hasMetadata && (
                <MetadataContainer>
                    <CompactMetadataRow>
                        {containerPorts && containerPorts.length > 0 && (
                            <div className="flex items-center gap-1">
                                <MetadataLabel>ports</MetadataLabel>
                                <ContainerPortList containerPorts={containerPorts} highlightedPorts={targetPorts} highlightedPortNames={targetPortNames} />
                            </div>
                        )}
                        {podIps && podIps.length > 0 && <MetadataItem label="ips" value={podIps} />}
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
