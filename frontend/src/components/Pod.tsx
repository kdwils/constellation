import { LabelDisplay } from "./LabelDisplay";
import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";
import { PortList } from "./PortList";

interface PodBoxProps {
    name: string;
    labels?: Record<string, string>;
    ports?: number[];
    serviceSelectors?: Record<string, string>;
    phase?: string;
}

export function PodBox({ name, labels, ports, serviceSelectors, phase }: PodBoxProps) {
    const hasMetadata = (labels && Object.keys(labels).length > 0) || (ports && ports.length > 0);

    return (
        <ResourceBox borderColor="border-cyan-500" marginLeft="ml-12">
            <ResourceHeader name={name} type="POD" dotColor="bg-cyan-500" phase={phase} />
            {hasMetadata && (
                <div className="mt-2 ml-5 space-y-1 text-xs">
                    {labels && Object.keys(labels).length > 0 && (
                        <MetadataRow icon="labels" alignItems="start">
                            <div className="flex-1 text-slate-600 font-medium">
                                <LabelDisplay labels={labels} highlightedLabels={serviceSelectors} />
                            </div>
                        </MetadataRow>
                    )}
                    {ports && ports.length > 0 && (
                        <MetadataRow icon="ports">
                            <PortList ports={ports} />
                        </MetadataRow>
                    )}
                </div>
            )}
        </ResourceBox>
    );
}
