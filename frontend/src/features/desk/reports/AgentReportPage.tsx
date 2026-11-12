import { Card, Skeleton, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { getRouteApi, Link } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";
import { getAgentReport } from "#/api/endpoints/desk-reports/desk-reports";
import { deskKeys } from "#/api/query-keys";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError } from "#/components/ui";
import { useI18n } from "#/i18n";
import { ExportButtons, ReportTableView } from "./ReportTableView";

const route = getRouteApi("/_app/desk/reports/agents");

/** The agents' work in this week, month and 3 months - or the periods just before them. */
export function AgentReportPage() {
	const search = route.useSearch();
	const navigate = route.useNavigate();
	const { t, lang } = useI18n();
	const previous = Boolean(search.previous);
	const report = useQuery({
		queryKey: deskKeys.report("agents", { previous, lang }),
		queryFn: () => getAgentReport({ previous, lang }),
	});
	const query = `previous=${previous}&lang=${lang}`;
	return (
		<>
			<PageHeader
				eyebrow={
					<Link to="/desk/reports" className="inline-flex items-center gap-1 hover:text-foreground">
						<ArrowLeft className="size-3" /> {t("Raporty", "Reports")}
					</Link>
				}
				title={t("Praca agentów", "Agents' work")}
				description={t(
					"Liczone ze zdarzeń: przekazane zgłoszenie liczy się temu, kto je naprawdę przejął lub zrealizował.",
					"Counted from the events: a handed-over ticket counts for whoever really took or resolved it.",
				)}
				actions={<ExportButtons href={(f) => `/api/desk/reports/agents/export.${f}?${query}`} />}
			/>
			<Card className="overflow-hidden p-0">
				<div className="border-b border-separator p-4">
					<ToggleButtonGroup
						aria-label={t("Okresy", "Periods")}
						selectionMode="single"
						disallowEmptySelection
						size="sm"
						selectedKeys={new Set([previous ? "previous" : "current"])}
						onSelectionChange={(keys) =>
							void navigate({
								search: { previous: [...keys][0] === "previous" || undefined },
								replace: true,
							})
						}
					>
						<ToggleButton id="current">{t("Bieżące okresy", "Current periods")}</ToggleButton>
						<ToggleButton id="previous">{t("Poprzednie okresy", "Previous periods")}</ToggleButton>
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
