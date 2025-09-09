import { ResourceBox } from "./ResourceBox";
import { ResourceHeader } from "./ResourceHeader";

interface IngressProps {
    name: string;
}

export function IngressBox({ name }: IngressProps) {
    return (
        <ResourceBox borderColor="border-emerald-500" marginLeft="ml-4">
            <ResourceHeader name={name} type="INGRESS" dotColor="bg-emerald-500" />
        </ResourceBox>
    );
}