import { Card } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { UsersRound } from "lucide-react";
import { PageHeader } from "#/components/layout/AppLayout";
import { useI18n } from "#/i18n";
import { CategoryIcon } from "../components/CategoryIcon";
import { deskCatalogQuery } from "../hooks";
import { REPORTS } from "./search";

/** The reports: one per team that collects report data, and the agents' work. */
export function ReportsPage() {
	const { t, n } = useI18n();
	const catalog = useQuery(deskCatalogQuery());
	return (
		<>
			<PageHeader
				eyebrow={t("Service desk", "Service desk")}
				title={t("Raporty", "Reports")}
				description={t(
					"Zgłoszenia z danymi do raportu i praca agentów, z eksportem do XLSX, PDF i CSV.",
					"Tickets with their report data and the agents' work, exported to XLSX, PDF and CSV.",
				)}
			/>
			<div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
				{REPORTS.map((r) => {
					const category = catalog.data?.categories.find((c) => c.key === r.categoryKey);
					return (
						<Link key={r.slug} to="/desk/reports/$slug" params={{ slug: r.slug }} search={{}}>
							<Card className="h-full transition-colors hover:bg-surface-secondary">
								<Card.Content className="flex flex-col gap-3">
									<CategoryIcon categoryKey={r.categoryKey} />
									<span className="font-medium">{n(category?.name) || r.slug}</span>
									<span className="text-xs text-muted">
										{category
											? t(
													`${category.subcategories.reduce((sum, s) => sum + s.fields.length, 0)} pól w tematach`,
													`${category.subcategories.reduce((sum, s) => sum + s.fields.length, 0)} fields in the topics`,
												)
											: ""}
									</span>
								</Card.Content>
							</Card>
						</Link>
					);
				})}
				<Link to="/desk/reports/agents" search={{}}>
					<Card className="h-full transition-colors hover:bg-surface-secondary">
						<Card.Content className="flex flex-col gap-3">
							<span className="flex size-9 items-center justify-center rounded-lg bg-accent-soft text-ink">
								<UsersRound className="size-4" strokeWidth={1.75} />
							</span>
							<span className="font-medium">{t("Praca agentów", "Agents' work")}</span>
							<span className="text-xs text-muted">
								{t("Przejęte, zrealizowane i po terminie", "Taken, resolved and late")}
							</span>
						</Card.Content>
					</Card>
				</Link>
			</div>
		</>
	);
}
