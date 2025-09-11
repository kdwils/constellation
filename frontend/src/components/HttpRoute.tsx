import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";
import { LabelList } from "./LabelList";
import { HighlightableList } from "./HighlightableList";
import { MetadataContainer } from "./MetadataContainer";

interface HttpRouteProps {
    name: string;
    hostnames?: string[];
    backend_refs?: string[];
    referencedServiceNames?: string[];
}

export function HttpRouteBox({ name, hostnames, backend_refs, referencedServiceNames = [] }: HttpRouteProps) {
    const hasMetadata = (hostnames && hostnames.length > 0) || (backend_refs && backend_refs.length > 0);

    return (
        <ResourceBox borderColor="border-violet-300" marginLeft="ml-0">
            <ResourceHeader name={name} type="HTTP ROUTE" dotColor="bg-violet-500" />
            {hasMetadata && (
                <MetadataContainer>
                    {hostnames && hostnames.length > 0 && (
                        <MetadataRow icon="hostnames">
                            <LabelList items={hostnames} />
                        </MetadataRow>
                    )}
                    {backend_refs && backend_refs.length > 0 && (
                        <MetadataRow icon="refs">
                            <HighlightableList
                                items={backend_refs}
                                getDisplayText={(ref) => ref}
                                isHighlighted={(ref) => referencedServiceNames.includes(ref)}
                            />
                        </MetadataRow>
                    )}
                </MetadataContainer>
            )}
        </ResourceBox>
    );
}
