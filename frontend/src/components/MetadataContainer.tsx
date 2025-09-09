interface MetadataContainerProps {
    children: React.ReactNode;
}

export function MetadataContainer({ children }: MetadataContainerProps) {
    return (
        <div className="mt-2 ml-5 space-y-1 text-xs">
            {children}
        </div>
    );
}