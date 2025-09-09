import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";
import { MetadataRow } from "./MetadataRow";

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
                <div className="mt-2 ml-5 space-y-1 text-xs">
                    {hostnames && hostnames.length > 0 && (
                        <MetadataRow icon="hostnames">
                            <span className="text-slate-600 font-medium">{hostnames.join(', ')}</span>
                        </MetadataRow>
                    )}
                    {backend_refs && backend_refs.length > 0 && (
                        <MetadataRow icon="refs">
                            <div className="flex flex-wrap gap-x-2 gap-y-1">
                                {backend_refs.map((serviceName, index) => (
                                    <span key={index} className="text-slate-600 font-medium px-2 py-0.5 bg-amber-100 border border-amber-300 rounded">
                                        {serviceName}
                                    </span>
                                ))}
                            </div>
                        </MetadataRow>
                    )}
                </div>
            )}
        </ResourceBox>
    );
}
