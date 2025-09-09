interface CompactMetadataRowProps {
    children: React.ReactNode;
}

export function CompactMetadataRow({ children }: CompactMetadataRowProps) {
    return (
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            {children}
        </div>
    );
}