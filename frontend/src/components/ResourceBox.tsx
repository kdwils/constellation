import { type ReactNode } from "react";

interface ResourceBoxProps {
    children: ReactNode;
    borderColor: string;
    marginLeft: string;
}

export function ResourceBox({ children, borderColor, marginLeft }: ResourceBoxProps) {
    return (
        <div className={`${marginLeft} p-3 bg-white border-l-4 ${borderColor} rounded-r-md shadow-sm`}>
            {children}
        </div>
    );
}