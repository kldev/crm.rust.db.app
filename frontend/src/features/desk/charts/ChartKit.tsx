import { Button, Card } from "@heroui/react";
import type { LucideIcon } from "lucide-react";
import { type ReactNode, useId, useRef, useState } from "react";
import { useI18n } from "#/i18n";

/*
 * Charts drawn as SVG without a chart library (ADR-0011), to the dataviz mark specs: thin bars (<= 24px) with a 4px
 * rounded data end and a 2px surface gap between segments, 2px lines, 8px markers with a surface ring, hairline solid
 * gridlines, one axis. Every chart has a hover tooltip and a table view; colour is never the only signal (legends,
 * status icons and labels). Text uses text tokens, never the series colour.
 */

export interface Segment {
	key: string;
	label: string;
	color: string;
	icon?: LucideIcon;
}

/** A chart card: title, one-line description, the chart and a "show table" toggle with the same numbers. */
export function ChartCard({
	title,
	description,
	legend,
	table,
	children,
}: {
	title: string;
	description?: string;
	legend?: ReactNode;
	table: ReactNode;
	children: ReactNode;
}) {
	const { t } = useI18n();
	const [showTable, setShowTable] = useState(false);
	return (
		<Card>
			<Card.Header className="flex-row items-start justify-between gap-3">
				<div>
					<Card.Title>{title}</Card.Title>
					{description ? <Card.Description>{description}</Card.Description> : null}
				</div>
				<Button size="sm" variant="ghost" onPress={() => setShowTable((v) => !v)}>
					{showTable ? t("Pokaż wykres", "Show chart") : t("Pokaż tabelę", "Show table")}
				</Button>
			</Card.Header>
			<Card.Content className="flex flex-col gap-3">
				{showTable ? table : children}
				{!showTable && legend ? legend : null}
			</Card.Content>
		</Card>
	);
}

export function Legend({ items, line }: { items: Segment[]; line?: boolean }) {
	return (
		<ul className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted">
			{items.map((s) => (
				<li key={s.key} className="inline-flex items-center gap-1.5">
					{line ? (
						<span className="inline-block h-0.5 w-4 rounded" style={{ background: s.color }} />
					) : (
						<span className="inline-block size-2.5 rounded-sm" style={{ background: s.color }} />
					)}
					{s.icon ? <s.icon className="size-3" /> : null}
					{s.label}
				</li>
			))}
		</ul>
	);
}

export function StatTile({
	label,
	value,
	note,
	icon: Icon,
	tone,
}: {
	label: string;
	value: string;
	note?: string;
	icon?: LucideIcon;
	tone?: "danger" | "warning";
}) {
	return (
		<Card>
			<Card.Content className="flex flex-col gap-1">
				<span className="inline-flex items-center gap-1.5 text-xs text-muted">
					{Icon ? (
						<Icon
							className={`size-3.5 ${tone === "danger" ? "text-danger" : tone === "warning" ? "text-warning" : ""}`}
						/>
					) : null}
					{label}
				</span>
				<span className="font-display text-3xl font-semibold text-foreground">{value}</span>
				{note ? <span className="text-xs text-muted">{note}</span> : null}
			</Card.Content>
		</Card>
	);
}

/** A floating readout; the value leads, the label follows (dataviz interaction rules). */
function Tooltip({ x, y, children }: { x: number | string; y: number; children: ReactNode }) {
	return (
		<div
			role="status"
			className="pointer-events-none absolute z-10 min-w-36 -translate-x-1/2 -translate-y-full rounded-md border border-separator bg-surface px-3 py-2 text-xs shadow-md"
			style={{ left: x, top: y - 8 }}
		>
			{children}
		</div>
	);
}

function TipRow({ color, value, label }: { color: string; value: ReactNode; label: string }) {
	return (
		<div className="flex items-center gap-2">
			<span className="inline-block h-0.5 w-3 rounded" style={{ background: color }} />
			<strong className="figures text-foreground">{value}</strong>
			<span className="text-muted">{label}</span>
		</div>
	);
}

/** Round "nice" ticks from 0 to at least `max`. */
function ticks(max: number): number[] {
	if (max <= 0) return [0, 1];
	const rough = max / 4;
	const power = 10 ** Math.floor(Math.log10(rough));
	const step = [1, 2, 5, 10].map((m) => m * power).find((s) => s >= rough) ?? rough;
	const out: number[] = [];
	for (let v = 0; v <= max + step * 0.001; v += step) out.push(Math.round(v * 1000) / 1000);
	if (out[out.length - 1] < max) out.push(out[out.length - 1] + step);
	return out;
}

const W = 640;

/** Every n-th label, and the last one only when it does not crowd the one before. */
function showLabel(i: number, count: number, every: number) {
	if (i === count - 1) return true;
	return i % every === 0 && count - 1 - i >= every / 2;
}

/** Lines over time with a crosshair that snaps to the nearest bucket and lists every series. */
export function LineChart({
	labels,
	series,
	height = 220,
}: {
	labels: string[];
	series: (Segment & { values: number[] })[];
	height?: number;
}) {
	const id = useId();
	const [hover, setHover] = useState<number | null>(null);
	const pad = { left: 36, right: 12, top: 12, bottom: 28 };
	const max = Math.max(1, ...series.flatMap((s) => s.values));
	const yTicks = ticks(max);
	const top = yTicks[yTicks.length - 1];
	const iw = W - pad.left - pad.right;
	const ih = height - pad.top - pad.bottom;
	const x = (i: number) =>
		pad.left + (labels.length <= 1 ? iw / 2 : (i * iw) / (labels.length - 1));
	const y = (v: number) => pad.top + ih - (v / top) * ih;
	const every = Math.ceil(labels.length / 8);

	return (
		<div className="relative">
			<svg
				viewBox={`0 0 ${W} ${height}`}
				className="w-full"
				role="img"
				aria-labelledby={`${id}-desc`}
				onPointerLeave={() => setHover(null)}
				onPointerMove={(e) => {
					const box = e.currentTarget.getBoundingClientRect();
					const px = ((e.clientX - box.left) / box.width) * W;
					const i = Math.round(((px - pad.left) / iw) * (labels.length - 1));
					setHover(Math.max(0, Math.min(labels.length - 1, i)));
				}}
			>
				<title id={`${id}-desc`}>{series.map((s) => s.label).join(", ")}</title>
				{yTicks.map((v) => (
					<g key={v}>
						<line
							x1={pad.left}
							x2={W - pad.right}
							y1={y(v)}
							y2={y(v)}
							stroke="var(--viz-grid)"
							strokeWidth={1}
						/>
						<text x={pad.left - 6} y={y(v) + 3} textAnchor="end" className="fill-muted text-[10px]">
							{v}
						</text>
					</g>
				))}
				{labels.map((l, i) =>
					showLabel(i, labels.length, every) ? (
						<text
							key={`${l}-${i}`}
							x={x(i)}
							y={height - 8}
							textAnchor="middle"
							className="fill-muted text-[10px]"
						>
							{l}
						</text>
					) : null,
				)}
				{hover != null ? (
					<line
						x1={x(hover)}
						x2={x(hover)}
						y1={pad.top}
						y2={pad.top + ih}
						stroke="var(--viz-neutral)"
						strokeWidth={1}
					/>
				) : null}
				{series.map((s) => (
					<g key={s.key}>
						<polyline
							fill="none"
							stroke={s.color}
							strokeWidth={2}
							strokeLinejoin="round"
							strokeLinecap="round"
							points={s.values.map((v, i) => `${x(i)},${y(v)}`).join(" ")}
						/>
						{hover != null ? (
							<circle
								cx={x(hover)}
								cy={y(s.values[hover] ?? 0)}
								r={4}
								fill={s.color}
								stroke="var(--surface)"
								strokeWidth={2}
							/>
						) : null}
					</g>
				))}
			</svg>
			{hover != null ? (
				<Tooltip x={`${(x(hover) / W) * 100}%`} y={0}>
					<div className="mb-1 text-muted">{labels[hover]}</div>
					{series.map((s) => (
						<TipRow key={s.key} color={s.color} value={s.values[hover]} label={s.label} />
					))}
				</Tooltip>
			) : null}
		</div>
	);
}

/**
 * Horizontal stacked bars, one row per category; each segment is its own hover target. A 2px surface gap separates
 * the segments; the row total is a direct label at the end.
 */
export function StackedBars({
	rows,
	segments,
}: {
	rows: { key: string; label: string; values: number[] }[];
	segments: Segment[];
}) {
	const [hover, setHover] = useState<{ row: number; seg: number; x: number; y: number } | null>(
		null,
	);
	const box = useRef<HTMLDivElement>(null);
	const max = Math.max(1, ...rows.map((r) => r.values.reduce((a, b) => a + b, 0)));
	/** The tooltip sits above the hovered or focused segment. */
	const show = (row: number, seg: number, mark: HTMLElement) => {
		const at = mark.getBoundingClientRect();
		const origin = box.current?.getBoundingClientRect() ?? at;
		setHover({ row, seg, x: at.left - origin.left + at.width / 2, y: at.top - origin.top });
	};
	return (
		<div ref={box} className="relative flex flex-col gap-2" onPointerLeave={() => setHover(null)}>
			{rows.map((row, r) => {
				const total = row.values.reduce((a, b) => a + b, 0);
				return (
					<div
						key={row.key}
						className="grid grid-cols-[minmax(7rem,12rem)_1fr_2.5rem] items-center gap-3"
					>
						<span className="truncate text-xs text-foreground" title={row.label}>
							{row.label}
						</span>
						<div className="flex h-5 gap-0.5" style={{ width: `${(total / max) * 100}%` }}>
							{row.values.map((v, s) =>
								v > 0 ? (
									<button
										key={segments[s].key}
										type="button"
										aria-label={`${row.label}: ${segments[s].label} ${v}`}
										className="h-full first:rounded-l-none last:rounded-r-[4px] hover:opacity-80 focus-visible:outline-2"
										style={{ flexGrow: v, background: segments[s].color }}
										onPointerEnter={(e) => show(r, s, e.currentTarget)}
										onFocus={(e) => show(r, s, e.currentTarget)}
										onBlur={() => setHover(null)}
									/>
								) : null,
							)}
						</div>
						<span className="figures text-right text-xs text-muted">{total}</span>
					</div>
				);
			})}
			{hover ? (
				<Tooltip x={hover.x} y={hover.y}>
					<div className="mb-1 text-muted">{rows[hover.row].label}</div>
					<TipRow
						color={segments[hover.seg].color}
						value={rows[hover.row].values[hover.seg]}
						label={segments[hover.seg].label}
					/>
				</Tooltip>
			) : null}
		</div>
	);
}

/** Medians of "time to resolve / SLA" per priority, with the p90 as a tick and the whole SLA (100%) as a line. */
export function SlaChart({
	rows,
	color,
	medianLabel,
	p90Label,
}: {
	rows: { key: string; label: string; median: number | null; p90: number | null; note: string }[];
	color: string;
	medianLabel: string;
	p90Label: string;
}) {
	const [hover, setHover] = useState<number | null>(null);
	// Capped at 300% of the SLA: one very late priority must not flatten the others; past it the bar ends in a notch.
	const CAP = 3;
	const max = Math.min(CAP, Math.max(1.2, ...rows.flatMap((r) => [r.median ?? 0, r.p90 ?? 0])));
	const pct = (v: number) => `${(Math.min(v, max) / max) * 100}%`;
	return (
		<div className="flex flex-col gap-3">
			{rows.map((row, i) => (
				<div key={row.key} className="grid grid-cols-[6rem_1fr_6rem] items-center gap-3">
					<span className="text-xs text-foreground">{row.label}</span>
					<div
						className="relative h-5"
						onPointerEnter={() => setHover(i)}
						onPointerLeave={() => setHover(null)}
						title={row.median == null ? "" : `${medianLabel} ${Math.round(row.median * 100)}%`}
					>
						<span
							className="absolute inset-y-0 w-px bg-[var(--viz-critical)]"
							style={{ left: pct(1) }}
							aria-hidden
						/>
						{row.median != null ? (
							<span
								className="absolute inset-y-0.5 left-0 rounded-r-[4px]"
								style={{
									width: pct(row.median),
									background: color,
									opacity: hover === i ? 0.8 : 1,
								}}
							/>
						) : null}
						{row.median != null && row.median > max ? (
							<span
								className="absolute -right-1 inset-y-0 flex items-center text-[10px] font-semibold text-foreground"
								aria-hidden
							>
								»
							</span>
						) : null}
						{row.p90 != null ? (
							<span
								className="absolute inset-y-0 w-0.5 bg-foreground"
								style={{ left: pct(row.p90) }}
								title={`${p90Label} ${Math.round(row.p90 * 100)}%`}
							/>
						) : null}
					</div>
					<span className="text-right text-xs text-muted">{row.note}</span>
				</div>
			))}
		</div>
	);
}

/** Weekday x hour counts in one sequential hue: light is few, dark is many; each cell has its own tooltip. */
export function Heatmap({
	rows,
	rowLabels,
	title,
}: {
	rows: number[][];
	rowLabels: string[];
	title: (r: number, h: number, v: number) => string;
}) {
	const max = Math.max(1, ...rows.flat());
	const step = (v: number) => (v === 0 ? 0 : Math.min(7, 1 + Math.floor((v / max) * 6.999)));
	return (
		<div className="overflow-x-auto">
			<div className="grid min-w-[560px] max-w-5xl grid-cols-[2.5rem_repeat(24,minmax(0,1fr))] gap-0.5 text-[10px] text-muted">
				<span />
				{Array.from({ length: 24 }, (_, h) => (
					<span key={h} className="text-center">
						{h % 3 === 0 ? h : ""}
					</span>
				))}
				{rows.map((row, r) => (
					<div key={rowLabels[r]} className="contents">
						<span className="self-center">{rowLabels[r]}</span>
						{row.map((v, h) => (
							<span
								key={h}
								className="h-5 rounded-[3px] hover:outline hover:outline-1 hover:outline-foreground"
								style={{ background: `var(--viz-seq-${step(v)})` }}
								title={title(r, h, v)}
							/>
						))}
					</div>
				))}
			</div>
		</div>
	);
}

/** A plain table with the numbers of a chart. */
export function DataTable({ head, rows }: { head: string[]; rows: (string | number)[][] }) {
	return (
		<div className="overflow-x-auto">
			<table className="w-full text-xs">
				<thead className="text-muted">
					<tr>
						{head.map((h) => (
							<th key={h} className="border-b border-separator px-2 py-1.5 text-left font-medium">
								{h}
							</th>
						))}
					</tr>
				</thead>
				<tbody>
					{rows.map((row, i) => (
						<tr key={`${row[0]}-${i}`} className="border-b border-separator/60">
							{row.map((cell, j) => (
								<td
									key={`${head[j]}`}
									className={`px-2 py-1.5 ${typeof cell === "number" ? "figures text-right" : ""}`}
								>
									{cell}
								</td>
							))}
						</tr>
					))}
				</tbody>
			</table>
		</div>
	);
}

/**
 * Stacked columns over time, one per bucket: <= 24px thick, a 2px surface gap between segments, a 4px rounded top.
 * Each segment is its own hover target; the readout names the bucket and the segment.
 */
export function StackedColumns({
	labels,
	segments,
	values,
	height = 220,
}: {
	labels: string[];
	segments: Segment[];
	/** values[bucket][segment] */
	values: number[][];
	height?: number;
}) {
	const id = useId();
	const [hover, setHover] = useState<{ b: number; s: number } | null>(null);
	const pad = { left: 36, right: 12, top: 12, bottom: 28 };
	const totals = values.map((v) => v.reduce((a, b) => a + b, 0));
	const yTicks = ticks(Math.max(1, ...totals));
	const top = yTicks[yTicks.length - 1];
	const iw = W - pad.left - pad.right;
	const ih = height - pad.top - pad.bottom;
	const slot = iw / Math.max(1, labels.length);
	const bar = Math.min(24, slot * 0.6);
	const y = (v: number) => pad.top + ih - (v / top) * ih;
	const every = Math.ceil(labels.length / 8);
	return (
		<div className="relative" onPointerLeave={() => setHover(null)}>
			<svg viewBox={`0 0 ${W} ${height}`} className="w-full" role="img" aria-labelledby={`${id}-t`}>
				<title id={`${id}-t`}>{segments.map((s) => s.label).join(", ")}</title>
				{yTicks.map((v) => (
					<g key={v}>
						<line
							x1={pad.left}
							x2={W - pad.right}
							y1={y(v)}
							y2={y(v)}
							stroke="var(--viz-grid)"
							strokeWidth={1}
						/>
						<text x={pad.left - 6} y={y(v) + 3} textAnchor="end" className="fill-muted text-[10px]">
							{v}
						</text>
					</g>
				))}
				{values.map((row, b) => {
					const x = pad.left + slot * b + (slot - bar) / 2;
					let base = 0;
					const last = row.reduce((k, v, i) => (v > 0 ? i : k), -1);
					return (
						<g key={labels[b]}>
							{row.map((v, s) => {
								if (v <= 0) return null;
								const y0 = y(base);
								base += v;
								// 2px surface gap between stacked segments; the top one gets the rounded end.
								const h = Math.max(1, y0 - y(base) - (s === last ? 0 : 2));
								return (
									<rect
										key={segments[s].key}
										x={x}
										y={y(base)}
										width={bar}
										height={h}
										rx={s === last ? 4 : 0}
										fill={segments[s].color}
										opacity={hover && (hover.b !== b || hover.s !== s) ? 0.55 : 1}
										onPointerEnter={() => setHover({ b, s })}
									>
										<title>{`${labels[b]}: ${segments[s].label} ${v}`}</title>
									</rect>
								);
							})}
							{showLabel(b, labels.length, every) ? (
								<text
									x={x + bar / 2}
									y={height - 8}
									textAnchor="middle"
									className="fill-muted text-[10px]"
								>
									{labels[b]}
								</text>
							) : null}
						</g>
					);
				})}
			</svg>
			{hover ? (
				<Tooltip x={`${((pad.left + slot * hover.b + slot / 2) / W) * 100}%`} y={0}>
					<div className="mb-1 text-muted">{labels[hover.b]}</div>
					{segments.map((s, i) => (
						<TipRow key={s.key} color={s.color} value={values[hover.b][i]} label={s.label} />
					))}
				</Tooltip>
			) : null}
		</div>
	);
}
