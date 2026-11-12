import type { ReactNode } from "react";

/** Label / value pairs of a details card. */
export function InfoList({ items }: { items: { label: string; value: ReactNode }[] }) {
	return (
		<dl className="grid grid-cols-[minmax(110px,auto)_1fr] gap-x-6 gap-y-3 text-sm">
			{items.map((item) => (
				<div key={item.label} className="contents">
					<dt className="font-mono text-[11px] tracking-[0.08em] text-muted uppercase">
						{item.label}
					</dt>
					<dd className="min-w-0 break-words text-foreground">{item.value}</dd>
				</div>
			))}
		</dl>
	);
}
