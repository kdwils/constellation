import { LabelDisplay } from "./LabelDisplay";
import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";
import { PortList } from "./PortList";

interface ServiceBoxProps {
    name: string;
    selectors?: Record<string, string>;
    ports?: number[];
    isTargetedByRoute?: boolean;
}

export function ServiceBox({ name, selectors, ports, isTargetedByRoute }: ServiceBoxProps) {
    const hasMetadata = (selectors && Object.keys(selectors).length > 0) || (ports && ports.length > 0);

    return (
        <ResourceBox borderColor={isTargetedByRoute ? "border-amber-400 border-2" : "border-amber-500"} marginLeft="ml-8">
            <ResourceHeader name={name} type="SERVICE" dotColor="bg-amber-500" />
            {hasMetadata && (
                <div className="mt-2 ml-5 space-y-1 text-xs">
                    {selectors && Object.keys(selectors).length > 0 && (
                        <MetadataRow icon="selectors" alignItems="start">
                            <div className="flex-1 text-slate-600 font-medium">
                                <LabelDisplay labels={selectors} alwaysHighlight={true} />
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
