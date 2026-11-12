import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

export function EmptyState({
	icon: Icon,
	title,
	description,
	action,
}: {
	icon: LucideIcon;
	title: string;
	description?: ReactNode;
	action?: ReactNode;
}) {
	return (
		<div className="flex flex-col items-center justify-center gap-3 px-6 py-14 text-center">
			<div className="flex size-12 items-center justify-center rounded-full bg-accent-soft text-ink">
				<Icon className="size-5" strokeWidth={1.75} />
			</div>
			<p className="font-display text-xl font-semibold text-foreground">{title}</p>
			{description ? <p className="max-w-sm text-sm text-muted">{description}</p> : null}
			{action}
		</div>
	);
}
