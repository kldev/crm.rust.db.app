import { Skeleton } from "@heroui/react";

export function TableSkeleton({ rows = 8 }: { rows?: number }) {
	return (
		<div className="flex flex-col gap-3 p-4">
			{Array.from({ length: rows }, (_, i) => (
				<Skeleton key={i} className="h-9 w-full rounded-md" />
			))}
		</div>
	);
}
