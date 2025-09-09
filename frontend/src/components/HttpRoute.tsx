import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";
import { LabelList } from "./LabelList";
import { MetadataContainer } from "./MetadataContainer";

interface HttpRouteProps {
    name: string;
    hostnames?: string[];
    backend_refs?: string[];
}

export function HttpRouteBox({ name, hostnames, backend_refs }: HttpRouteProps) {
    const hasMetadata = (hostnames && hostnames.length > 0) || (backend_refs && backend_refs.length > 0);

    return (
        <ResourceBox borderColor="border-violet-500" marginLeft="ml-4">
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
                            <div className="flex items-center gap-1">
                                <LabelList items={backend_refs} />
                            </div>
                        </MetadataRow>
                    )}
                </MetadataContainer>
            )}
        </ResourceBox>
    );
}
