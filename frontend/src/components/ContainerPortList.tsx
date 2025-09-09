import { LabelBadge } from "./LabelBadge";
import type { ContainerPortInfo } from "../ResourceNode";

interface ContainerPortListProps {
    containerPorts: ContainerPortInfo[];
    highlightedPorts?: number[];
    highlightedPortNames?: string[];
}

export function ContainerPortList({ containerPorts, highlightedPorts = [], highlightedPortNames = [] }: ContainerPortListProps) {
    return (
        <div className="flex flex-wrap gap-x-2 gap-y-1">
            {containerPorts.map((portInfo, index) => {
                const isHighlighted = highlightedPorts.includes(portInfo.port) || 
                    (portInfo.name ? highlightedPortNames.includes(portInfo.name) : false);
                    
                const displayText = portInfo.name 
                    ? `${portInfo.port} (${portInfo.name})`
                    : portInfo.port.toString();
                
                return (
                    <LabelBadge key={index} highlighted={isHighlighted}>
                        {displayText}
                    </LabelBadge>
                );
            })}
        </div>
    );
}