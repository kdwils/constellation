import { type ReactNode } from "react";

interface ResourceBoxProps {
    children: ReactNode;
    borderColor: string;
    marginLeft: string;
}

export function ResourceBox({ children, borderColor, marginLeft }: ResourceBoxProps) {
    return (
        <div className={`${marginLeft} p-4 bg-white border ${borderColor} rounded-lg shadow-sm hover:shadow-md transition-shadow duration-200`}>
            {children}
        </div>
    );
}