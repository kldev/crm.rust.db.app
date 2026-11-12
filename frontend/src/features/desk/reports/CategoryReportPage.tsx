import {
	Card,
	Input,
	Label,
	Skeleton,
	TextField,
	ToggleButton,
	ToggleButtonGroup,
} from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { getRouteApi, Link } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";
import { getCategoryReport } from "#/api/endpoints/desk-reports/desk-reports";
import { deskKeys } from "#/api/query-keys";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, SelectFilter } from "#/components/ui";
import { useI18n } from "#/i18n";
import { deskCatalogQuery } from "../hooks";
import { ExportButtons, ReportTableView } from "./ReportTableView";
import { REPORTS, warsawToday } from "./search";

const route = getRouteApi("/_app/desk/reports/$slug");

/** A category report: the filters are in the URL, the table and its exports come from the same query. */
export function CategoryReportPage() {
	const { slug } = route.useParams();
	const search = route.useSearch();
	const navigate = route.useNavigate();
	const { t, n, lang } = useI18n();
	const catalog = useQuery(deskCatalogQuery());
	const today = warsawToday();
	const params = {
		from: search.from ?? `${today.slice(0, 8)}01`,
		to: search.to ?? today,
		market: search.market,
		subcategoryKey: search.sub,
		completion: search.completion,
		lang,
	};
	const report = useQuery({
		queryKey: deskKeys.report(slug, params),
		queryFn: () => getCategoryReport(slug, params),
	});
	const category = catalog.data?.categories.find(
		(c) => c.key === REPORTS.find((r) => r.slug === slug)?.categoryKey,
	);
	const markets = [
		...new Set(category?.subcategories.map((s) => s.market).filter((m): m is string => !!m) ?? []),
	];
	const set = (change: object) =>
		void navigate({ search: (s) => ({ ...s, ...change }), replace: true });
	const query = new URLSearchParams(
		Object.entries(params).filter((e): e is [string, string] => e[1] != null && e[1] !== ""),
	).toString();

	return (
		<>
			<PageHeader
				eyebrow={
					<Link to="/desk/reports" className="inline-flex items-center gap-1 hover:text-foreground">
						<ArrowLeft className="size-3" /> {t("Raporty", "Reports")}
					</Link>
				}
				title={report.data?.title ?? n(category?.name)}
				description={report.data?.subtitle}
				actions={
					<ExportButtons
						href={(f) => `/api/desk/reports/categories/${slug}/export.${f}?${query}`}
					/>
				}
			/>
			<Card className="overflow-hidden p-0">
				<div className="flex flex-col flex-wrap gap-3 border-b border-separator p-4 sm:flex-row sm:items-end">
					<TextField
						type="date"
						value={params.from}
						onChange={(v) => v && set({ from: v })}
						className="w-full sm:w-44"
					>
						<Label>{t("Od", "From")}</Label>
						<Input />
					</TextField>
					<TextField
						type="date"
						value={params.to}
						onChange={(v) => v && set({ to: v })}
						className="w-full sm:w-44"
					>
						<Label>{t("Do", "To")}</Label>
						<Input />
					</TextField>
					{markets.length > 0 ? (
						<SelectFilter
							label={t("Rynek", "Market")}
							value={search.market}
							options={markets.map((m) => ({ id: m, label: n(m) }))}
							onChange={(market) => set({ market, sub: undefined })}
						/>
					) : null}
					<SelectFilter
						label={t("Podkategoria", "Subcategory")}
						className="w-full sm:w-64"
						value={search.sub}
						options={(category?.subcategories ?? [])
							.filter((s) => !search.market || s.market === search.market)
							.map((s) => ({
								id: s.key,
								label: s.market ? `${n(s.market)} · ${n(s.name)}` : n(s.name),
							}))}
						onChange={(sub) => set({ sub })}
					/>
					<ToggleButtonGroup
						aria-label={t("Realizacja", "Completion")}
						selectionMode="single"
						size="sm"
						selectedKeys={new Set([search.completion ?? "ANY"])}
						onSelectionChange={(keys) => {
							const key = [...keys][0];
							set({ completion: key === "OPEN" || key === "DONE" ? key : undefined });
						}}
					>
						<ToggleButton id="ANY">{t("Wszystkie", "All")}</ToggleButton>
						<ToggleButton id="OPEN">{t("Otwarte", "Open")}</ToggleButton>
						<ToggleButton id="DONE">{t("Zrealizowane", "Resolved")}</ToggleButton>
					</ToggleButtonGroup>
				</div>
				{report.isPending ? (
					<Skeleton className="m-4 h-40 rounded-lg" />
				) : report.isError ? (
					<div className="p-4">
						<ApiError error={report.error} />
					</div>
				) : (
					<ReportTableView table={report.data} />
				)}
			</Card>
		</>
	);
}
