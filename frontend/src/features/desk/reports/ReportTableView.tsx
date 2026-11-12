import { Button } from "@heroui/react";
import { FileDown } from "lucide-react";
import type { ReportTableResponse } from "#/api/models";
import { TIME_ZONE, useI18n } from "#/i18n";

/**
 * A report table as the exports write it (ADR-0010): typed cells formatted for the reader, a second header row when
 * columns are grouped (the periods of the agents' report), totals in bold. Wide reports scroll sideways.
 */
export function ReportTableView({ table }: { table: ReportTableResponse }) {
	const { t, locale } = useI18n();
	const grouped = table.columns.some((c) => c.group);
	const groups: { label: string; span: number }[] = [];
	for (const c of table.columns) {
		const label = c.group ?? "";
		const last = groups.at(-1);
		if (last && last.label === label && label !== "") last.span += 1;
		else groups.push({ label, span: 1 });
	}
	const format = (kind: string, value: string | null | undefined) => {
		if (value == null || value === "") return "";
		switch (kind) {
			case "DATETIME":
				return new Intl.DateTimeFormat(locale, {
					timeZone: TIME_ZONE,
					dateStyle: "short",
					timeStyle: "short",
				}).format(new Date(value));
			case "DATE":
				return new Intl.DateTimeFormat(locale, { dateStyle: "short" }).format(
					new Date(`${value}T00:00:00`),
				);
			case "AMOUNT":
				return Number(value).toLocaleString(locale, {
					minimumFractionDigits: 2,
					maximumFractionDigits: 2,
				});
			default:
				return value;
		}
	};
	const numeric = (kind: string) => kind === "NUMBER" || kind === "AMOUNT";

	if (table.rows.length === 0)
		return (
			<p className="p-6 text-sm text-muted">
				{t("Brak zgłoszeń w tym okresie.", "No tickets in this period.")}
			</p>
		);
	return (
		<div className="overflow-x-auto">
			<table className="w-full min-w-max border-collapse text-sm">
				<thead className="bg-surface-secondary font-mono text-[11px] tracking-[0.06em] text-muted uppercase">
					{grouped ? (
						<tr>
							{groups.map((g, i) => (
								<th
									key={`${g.label}-${i}`}
									colSpan={g.span}
									className="border-b border-separator px-3 py-2 text-center"
								>
									{g.label}
								</th>
							))}
						</tr>
					) : null}
					<tr>
						{table.columns.map((c) => (
							<th
								key={c.key}
								className={`border-b border-separator px-3 py-2 whitespace-nowrap ${numeric(c.kind) ? "text-right" : "text-left"}`}
							>
								{c.label}
							</th>
						))}
					</tr>
				</thead>
				<tbody>
					{table.rows.map((row, r) => (
						<tr
							key={`${r}-${row[0]}`}
							className="border-b border-separator/60 hover:bg-surface-secondary/50"
						>
							{row.map((cell, i) => (
								<td
									key={table.columns[i]?.key ?? i}
									className={`px-3 py-2 align-top ${numeric(table.columns[i]?.kind ?? "") ? "figures text-right" : ""} ${
										table.columns[i]?.key === "title" ? "min-w-64" : "whitespace-nowrap"
									}`}
								>
									{format(table.columns[i]?.kind ?? "TEXT", cell)}
								</td>
							))}
						</tr>
					))}
					{table.totals ? (
						<tr className="font-semibold">
							{table.totals.map((cell, i) => (
								<td
									key={table.columns[i]?.key ?? i}
									className={`px-3 py-2 ${numeric(table.columns[i]?.kind ?? "") ? "figures text-right" : ""}`}
								>
									{format(table.columns[i]?.kind ?? "TEXT", cell)}
								</td>
							))}
						</tr>
					) : null}
				</tbody>
			</table>
		</div>
	);
}

/** CSV, XLSX and PDF of the same table: plain links, the session cookie authorises the download. */
export function ExportButtons({ href }: { href: (format: "csv" | "xlsx" | "pdf") => string }) {
	return (
		<div className="flex gap-2">
			{(["xlsx", "pdf", "csv"] as const).map((f) => (
				<a key={f} href={href(f)} download>
					<Button size="sm" variant="secondary">
						<FileDown className="size-3.5" />
						{f.toUpperCase()}
					</Button>
				</a>
			))}
		</div>
	);
}
