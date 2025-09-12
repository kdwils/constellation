import { HighlightableList } from "./HighlightableList";
import type { ContainerPortInfo } from "../types";

interface ContainerPortListProps {
    containerPorts: ContainerPortInfo[];
    highlightedPorts?: number[];
    highlightedPortNames?: string[];
}

export function ContainerPortList({ containerPorts, highlightedPorts = [], highlightedPortNames = [] }: ContainerPortListProps) {
    return (
        <HighlightableList
            items={containerPorts}
            getDisplayText={(portInfo) =>
                portInfo.name
                    ? `${portInfo.port} (${portInfo.name})`
                    : portInfo.port.toString()
            }
            isHighlighted={(portInfo) =>
                highlightedPorts.includes(portInfo.port) ||
                (portInfo.name ? highlightedPortNames.includes(portInfo.name) : false)
            }
        />
    );
}