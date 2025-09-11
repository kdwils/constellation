import { HighlightableList } from "./HighlightableList";
import type { ContainerPortInfo } from "../ResourceNode";

interface PortMappingListProps {
    portMappings: string[];
    childContainerPorts?: ContainerPortInfo[];
}

export function PortMappingList({ portMappings, childContainerPorts = [] }: PortMappingListProps) {
    const isPortHighlighted = (mapping: string): boolean => {
        const targetPortMatch = mapping.includes('→') 
            ? mapping.split('→')[1] 
            : mapping;
        
        return childContainerPorts.some(containerPort => {
            const targetPortNum = parseInt(targetPortMatch);
            if (!isNaN(targetPortNum) && containerPort.port === targetPortNum) {
                return true;
            }
            
            if (containerPort.name && containerPort.name === targetPortMatch) {
                return true;
            }
            
            return false;
        });
    };

    return (
        <HighlightableList
            items={portMappings}
            getDisplayText={(mapping) => mapping}
            isHighlighted={isPortHighlighted}
        />
    );
}