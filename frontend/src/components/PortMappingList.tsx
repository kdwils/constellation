import { LabelBadge } from "./LabelBadge";
import type { ContainerPortInfo } from "../ResourceNode";

interface PortMappingListProps {
    portMappings: string[];
    childContainerPorts?: ContainerPortInfo[];
}

export function PortMappingList({ portMappings, childContainerPorts = [] }: PortMappingListProps) {
    return (
        <div className="flex flex-wrap gap-x-2 gap-y-1">
            {portMappings.map((mapping, index) => {
                const targetPortMatch = mapping.includes('→') 
                    ? mapping.split('→')[1] 
                    : mapping;
                
                const isHighlighted = childContainerPorts.some(containerPort => {
                    const targetPortNum = parseInt(targetPortMatch);
                    if (!isNaN(targetPortNum) && containerPort.port === targetPortNum) {
                        return true;
                    }
                    
                    if (containerPort.name && containerPort.name === targetPortMatch) {
                        return true;
                    }
                    
                    return false;
                });
                
                return (
                    <LabelBadge key={index} highlighted={isHighlighted}>
                        {mapping}
                    </LabelBadge>
                );
            })}
        </div>
    );
}