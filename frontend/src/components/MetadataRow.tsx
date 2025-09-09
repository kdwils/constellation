import { type ReactNode } from "react";

interface MetadataRowProps {
    icon: string;
    children: ReactNode;
    alignItems?: "center" | "start";
}

export function MetadataRow({ icon, children, alignItems = "center" }: MetadataRowProps) {
    const alignmentClass = alignItems === "start" ? "items-start" : "items-center";
    const iconClass = alignItems === "start" ? "mt-0.5" : "";
    
    return (
        <div className={`flex ${alignmentClass} space-x-1`}>
            <span className={`text-slate-500 ${iconClass}`}>{icon}</span>
            {children}
        </div>
    );
}