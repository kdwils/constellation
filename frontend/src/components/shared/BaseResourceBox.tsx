import { ResourceBox } from "../ResourceBox";
import { ResourceHeader } from "../ResourceHeader";
import { MetadataContainer } from "../MetadataContainer";
import { CompactMetadataRow } from "../CompactMetadataRow";

interface BaseResourceBoxProps {
    name: string;
    type: string;
    borderColor: string;
    dotColor: string;
    phase?: string;
    children?: React.ReactNode;
    hasMetadata?: boolean;
    compactMetadata?: React.ReactNode;
    fullMetadata?: React.ReactNode;
}

export function BaseResourceBox({
    name,
    type,
    borderColor,
    dotColor,
    phase,
    children,
    hasMetadata = false,
    compactMetadata,
    fullMetadata
}: BaseResourceBoxProps) {
    return (
        <ResourceBox borderColor={borderColor} marginLeft="ml-0">
            <ResourceHeader name={name} type={type} dotColor={dotColor} phase={phase} />
            {hasMetadata && (
                <MetadataContainer>
                    {compactMetadata && (
                        <CompactMetadataRow>
                            {compactMetadata}
                        </CompactMetadataRow>
                    )}
                    {fullMetadata}
                </MetadataContainer>
            )}
            {children}
        </ResourceBox>
    );
}

// Resource-specific configurations
export const ResourceConfigs = {
    Service: {
        type: "SERVICE",
        borderColor: "border-amber-300",
        dotColor: "bg-amber-500",
        getBorderColor: (isTargeted: boolean) => 
            isTargeted ? "border-amber-400" : "border-amber-300"
    },
    
    Pod: {
        type: "POD",
        borderColor: "border-cyan-300",
        dotColor: "bg-cyan-500"
    },
    
    HTTPRoute: {
        type: "HTTP ROUTE",
        borderColor: "border-violet-300",
        dotColor: "bg-violet-500"
    },
    
    Ingress: {
        type: "INGRESS",
        borderColor: "border-purple-300",
        dotColor: "bg-purple-500"
    }
};