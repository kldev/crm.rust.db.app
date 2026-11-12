import { Skeleton, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { getRouteApi } from "@tanstack/react-router";
import { AlarmClock, CheckCircle2, Clock, Inbox } from "lucide-react";
import { getDeskCharts } from "#/api/endpoints/desk-charts/desk-charts";
import type { GetDeskChartsRange } from "#/api/models";
import { deskKeys } from "#/api/query-keys";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, SelectFilter } from "#/components/ui";
import { useI18n } from "#/i18n";
import { useCatalogNames } from "../components/useCatalogNames";
import { deskCatalogQuery } from "../hooks";
import { durationText, PRIORITIES, priorityLabel } from "../types";
import {
	ChartCard,
	DataTable,
	Heatmap,
	Legend,
	LineChart,
	type Segment,
	SlaChart,
	StackedBars,
	StackedColumns,
	StatTile,
} from "./ChartKit";

const route = getRouteApi("/_app/desk/charts");

const RANGES = ["DAYS_7", "DAYS_30", "WEEKS_12"] as const;

/**
 * The desk in charts (ADR-0011): flow in and out, the backlog, deadlines kept, SLA use per priority, by category, the
 * queue now, agents and the weekday x hour of reports. One row of filters scopes everything below it; while new numbers
 * load, the previous ones stay on screen.
 */
export function ChartsPage() {
	const search = route.useSearch();
	const navigate = route.useNavigate();
	const i18n = useI18n();
	const { t, locale } = i18n;
	const catalog = useQuery(deskCatalogQuery());
	const names = useCatalogNames(catalog.data);
	const range: GetDeskChartsRange = search.range ?? "DAYS_30";
	const params = { range, categoryKey: search.category };
	const charts = useQuery({
		queryKey: deskKeys.report("charts", params),
		queryFn: () => getDeskCharts(params),
		placeholderData: keepPreviousData,
	});

	const status: Segment[] = [
		{
			key: "ontime",
			label: t("W terminie", "On time"),
			color: "var(--viz-good)",
			icon: CheckCircle2,
		},
		{
			key: "late",
			label: t("Po terminie", "Late"),
			color: "var(--viz-critical)",
			icon: AlarmClock,
		},
	];
	const withOpen: Segment[] = [
		...status,
		{ key: "open", label: t("Otwarte", "Open"), color: "var(--viz-neutral)", icon: Inbox },
	];
	const queueSegments: Segment[] = [
		{
			key: "ontime",
			label: t("W terminie", "On time"),
			color: "var(--viz-good)",
			icon: CheckCircle2,
		},
		{ key: "soon", label: t("Kończy się", "Due soon"), color: "var(--viz-warning)", icon: Clock },
		{
			key: "overdue",
			label: t("Po terminie", "Overdue"),
			color: "var(--viz-critical)",
			icon: AlarmClock,
		},
	];
	const weekdays = t("pn,wt,śr,cz,pt,so,nd", "Mon,Tue,Wed,Thu,Fri,Sat,Sun").split(",");
	const day = (iso: string) =>
		new Intl.DateTimeFormat(locale, { day: "numeric", month: "short" }).format(
			new Date(`${iso}T00:00:00`),
		);

	return (
		<>
			<PageHeader
				eyebrow={t("Service desk", "Service desk")}
				title={t("Wykresy", "Charts")}
				description={
					charts.data
						? `${day(charts.data.from)} – ${day(charts.data.to)}`
						: t(
								"Jak płyną zgłoszenia i czy terminy są dotrzymane.",
								"How tickets flow and whether deadlines are kept.",
							)
				}
			/>
			<div className="mb-6 flex flex-col flex-wrap gap-3 sm:flex-row sm:items-center">
				<ToggleButtonGroup
					aria-label={t("Zakres", "Range")}
					selectionMode="single"
					disallowEmptySelection
					size="sm"
					selectedKeys={new Set([range])}
					onSelectionChange={(keys) => {
						const next = [...keys][0] as (typeof RANGES)[number];
						void navigate({
							search: (s) => ({ ...s, range: next === "DAYS_30" ? undefined : next }),
							replace: true,
						});
					}}
				>
					<ToggleButton id="DAYS_7">{t("7 dni", "7 days")}</ToggleButton>
					<ToggleButton id="DAYS_30">{t("30 dni", "30 days")}</ToggleButton>
					<ToggleButton id="WEEKS_12">{t("12 tygodni", "12 weeks")}</ToggleButton>
				</ToggleButtonGroup>
				<SelectFilter
					label={t("Kategoria", "Category")}
					className="w-full sm:w-64"
					value={search.category}
					options={(catalog.data?.categories ?? []).map((c) => ({
						id: c.key,
						label: i18n.n(c.name),
					}))}
					onChange={(category) =>
						void navigate({ search: (s) => ({ ...s, category }), replace: true })
					}
				/>
			</div>

			{charts.isPending ? (
				<Skeleton className="h-96 w-full rounded-xl" />
			) : charts.isError ? (
				<ApiError error={charts.error} />
			) : (
				(() => {
					const d = charts.data;
					const f = d.figures;
					const labels = d.flow.map((p) =>
						p.from === p.to ? day(p.from) : `${day(p.from)}–${day(p.to)}`,
					);
					const label = (key: string) =>
						d.bySubcategory ? names.subcategory(key) : names.category(key);
					return (
						<div className={`flex flex-col gap-6 ${charts.isPlaceholderData ? "opacity-60" : ""}`}>
							<div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
								<StatTile
									label={t("Zgłoszone", "Reported")}
									value={f.reported.toLocaleString(locale)}
									note={t(`zrealizowane: ${f.resolved}`, `resolved: ${f.resolved}`)}
								/>
								<StatTile
									label={t("Dotrzymane terminy", "Deadlines kept")}
									icon={CheckCircle2}
									value={f.onTimeShare == null ? "—" : `${Math.round(f.onTimeShare * 100)}%`}
									note={t(`po terminie: ${f.resolvedLate}`, `late: ${f.resolvedLate}`)}
								/>
								<StatTile
									label={t("Mediana realizacji", "Median time to resolve")}
									value={
										f.medianResolutionMinutes == null
											? "—"
											: durationText(f.medianResolutionMinutes * 60_000, i18n)
									}
								/>
								<StatTile
									label={t("Otwarte teraz", "Open now")}
									icon={AlarmClock}
									tone={f.overdueNow > 0 ? "danger" : undefined}
									value={f.openNow.toLocaleString(locale)}
									note={t(
										`po terminie: ${f.overdueNow}, kończy się: ${f.dueSoonNow}`,
										`overdue: ${f.overdueNow}, due soon: ${f.dueSoonNow}`,
									)}
								/>
							</div>

							<div className="grid gap-6 2xl:grid-cols-2">
								<ChartCard
									title={t("Napływ i realizacja", "Flow in and out")}
									description={t(
										"Zgłoszone i zrealizowane w każdym okresie.",
										"Reported and resolved in each period.",
									)}
									legend={
										<Legend
											line
											items={[
												{
													key: "rep",
													label: t("Zgłoszone", "Reported"),
													color: "var(--viz-series-1)",
												},
												{
													key: "res",
													label: t("Zrealizowane", "Resolved"),
													color: "var(--viz-series-2)",
												},
											]}
										/>
									}
									table={
										<DataTable
											head={[
												t("Okres", "Period"),
												t("Zgłoszone", "Reported"),
												t("Zrealizowane", "Resolved"),
											]}
											rows={d.flow.map((p, i) => [labels[i], p.reported, p.resolved])}
										/>
									}
								>
									<LineChart
										labels={labels}
										series={[
											{
												key: "rep",
												label: t("Zgłoszone", "Reported"),
												color: "var(--viz-series-1)",
												values: d.flow.map((p) => p.reported),
											},
											{
												key: "res",
												label: t("Zrealizowane", "Resolved"),
												color: "var(--viz-series-2)",
												values: d.flow.map((p) => p.resolved),
											},
										]}
									/>
								</ChartCard>

								<ChartCard
									title={t("Zaległości", "Backlog")}
									description={t(
										"Otwarte na koniec każdego okresu.",
										"Open at the end of each period.",
									)}
									table={
										<DataTable
											head={[t("Okres", "Period"), t("Otwarte", "Open")]}
											rows={d.flow.map((p, i) => [labels[i], p.openAtEnd])}
										/>
									}
								>
									<LineChart
										labels={labels}
										series={[
											{
												key: "open",
												label: t("Otwarte", "Open"),
												color: "var(--viz-series-1)",
												values: d.flow.map((p) => p.openAtEnd),
											},
										]}
									/>
								</ChartCard>

								<ChartCard
									title={t("Dotrzymane terminy", "Deadlines kept")}
									description={t(
										"Zrealizowane w terminie i po terminie w każdym okresie.",
										"Resolved on time and late in each period.",
									)}
									legend={<Legend items={status} />}
									table={
										<DataTable
											head={[t("Okres", "Period"), status[0].label, status[1].label]}
											rows={d.flow.map((p, i) => [
												labels[i],
												p.resolved - p.resolvedLate,
												p.resolvedLate,
											])}
										/>
									}
								>
									<StackedColumns
										labels={labels}
										segments={status}
										values={d.flow.map((p) => [p.resolved - p.resolvedLate, p.resolvedLate])}
									/>
								</ChartCard>

								<ChartCard
									title={t("Wykorzystanie SLA", "SLA use")}
									description={t(
										"Mediana czasu realizacji jako część SLA; kreska to 90. percentyl (od 5 zgłoszeń), czerwona linia to 100%.",
										"Median time to resolve as a share of the SLA; the tick is the 90th percentile (from 5 tickets), the red line 100%.",
									)}
									table={
										<DataTable
											head={[
												t("Priorytet", "Priority"),
												t("Liczba", "Count"),
												t("Mediana", "Median"),
												"p90",
												t("Mediana czasu", "Median time"),
											]}
											rows={d.sla.map((s) => [
												priorityLabel(s.priority, i18n),
												s.count,
												s.median == null ? "—" : `${Math.round(s.median * 100)}%`,
												s.p90 == null ? "—" : `${Math.round(s.p90 * 100)}%`,
												s.medianMinutes == null
													? "—"
													: durationText(s.medianMinutes * 60_000, i18n),
											])}
										/>
									}
								>
									<SlaChart
										color="var(--viz-series-1)"
										medianLabel={t("mediana", "median")}
										p90Label="p90"
										rows={PRIORITIES.map((p) => {
											const s = d.sla.find((x) => x.priority === p);
											return {
												key: p,
												label: priorityLabel(p, i18n),
												median: s?.median ?? null,
												p90: s?.p90 ?? null,
												note:
													s?.median == null ? "—" : `${Math.round(s.median * 100)}% · n=${s.count}`,
											};
										})}
									/>
								</ChartCard>

								<ChartCard
									title={
										d.bySubcategory
											? t("Według podkategorii", "By subcategory")
											: t("Według kategorii", "By category")
									}
									description={t(
										"Zgłoszone w zakresie: zrealizowane w terminie, po terminie i otwarte.",
										"Reported in the range: resolved on time, late and still open.",
									)}
									legend={<Legend items={withOpen} />}
									table={
										<DataTable
											head={[t("Kategoria", "Category"), ...withOpen.map((s) => s.label)]}
											rows={d.byCategory.map((b) => [label(b.key), ...b.values])}
										/>
									}
								>
									<StackedBars
										segments={withOpen}
										rows={d.byCategory.map((b) => ({
											key: b.key,
											label: label(b.key),
											values: b.values,
										}))}
									/>
								</ChartCard>

								<ChartCard
									title={t("Kolejka teraz", "The queue now")}
									description={t(
										"Otwarte zgłoszenia według priorytetu i stanu terminu.",
										"Open tickets by priority and deadline state.",
									)}
									legend={<Legend items={queueSegments} />}
									table={
										<DataTable
											head={[t("Priorytet", "Priority"), ...queueSegments.map((s) => s.label)]}
											rows={d.queue.map((b) => [priorityLabel(b.key, i18n), ...b.values])}
										/>
									}
								>
									<StackedBars
										segments={queueSegments}
										rows={d.queue.map((b) => ({
											key: b.key,
											label: priorityLabel(b.key, i18n),
											values: b.values,
										}))}
									/>
								</ChartCard>

								<ChartCard
									title={t("Agenci", "Agents")}
									description={t(
										"Zrealizowane w zakresie, w terminie i po terminie.",
										"Resolved in the range, on time and late.",
									)}
									legend={<Legend items={status} />}
									table={
										<DataTable
											head={[t("Agent", "Agent"), ...status.map((s) => s.label)]}
											rows={d.agents.map((b) => [agentLabel(b.key, b.label, t), ...b.values])}
										/>
									}
								>
									<StackedBars
										segments={status}
										rows={d.agents.map((b) => ({
											key: b.key || "?",
											label: agentLabel(b.key, b.label, t),
											values: b.values,
										}))}
									/>
								</ChartCard>

								<ChartCard
									title={t("Kiedy przychodzą zgłoszenia", "When tickets arrive")}
									description={t(
										"Zgłoszenia według dnia tygodnia i godziny (czas warszawski); ciemniej znaczy więcej.",
										"Tickets by weekday and hour (Warsaw time); darker means more.",
									)}
									table={
										<DataTable
											head={[t("Dzień", "Day"), ...Array.from({ length: 24 }, (_, h) => String(h))]}
											rows={d.heat.map((row, r) => [weekdays[r], ...row])}
										/>
									}
								>
									<Heatmap
										rows={d.heat}
										rowLabels={weekdays}
										title={(r, h, v) => `${weekdays[r]} ${h}:00 – ${v}`}
									/>
								</ChartCard>
							</div>
						</div>
					);
				})()
			)}
		</>
	);
}

function agentLabel(
	key: string,
	label: string | null | undefined,
	t: (pl: string, en: string) => string,
) {
	return key === "rest" ? t(`Pozostali (${label})`, `Others (${label})`) : (label ?? "?");
}
