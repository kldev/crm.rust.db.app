import { Button, Card, Skeleton } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { CheckCircle2, LogIn } from "lucide-react";
import { useState } from "react";
import { getCsrfToken } from "#/api/endpoints/auth/auth";
import type { PublicReportResponse } from "#/api/models";
import { ApiError } from "#/components/ui";
import { useI18n } from "#/i18n";
import { LanguageSwitch } from "#/i18n/LanguageSwitch";
import { ReportForm } from "./components/ReportForm";
import { publicCatalogQuery, publicPeopleQuery, useReportPublicTicket } from "./hooks";
import { formatInstant } from "./types";

/**
 * The public report form (no sign-in, ADR-0007): an employee reports a request to a team and gets its number.
 * The page shows names to pick from and creates tickets - nothing else.
 */
export function PublicReportPage() {
	const i18n = useI18n();
	const { t } = i18n;
	const catalog = useQuery(publicCatalogQuery());
	const people = useQuery(publicPeopleQuery());
	const report = useReportPublicTicket();
	const [sent, setSent] = useState<PublicReportResponse | null>(null);
	// A new form after "report another": remounting resets every field.
	const [round, setRound] = useState(0);

	return (
		<div className="min-h-screen bg-background">
			<header className="flex items-center gap-3 border-b border-separator bg-topbar px-4 py-3 sm:px-8">
				<img src="/favicon.svg" alt="" className="size-7" />
				<span className="font-display text-xl font-semibold">
					{t("Zgłoś sprawę", "Report a request")}
				</span>
				<div className="ml-auto flex items-center gap-3">
					<LanguageSwitch />
					<Link
						to="/login"
						className="inline-flex items-center gap-1 text-sm text-muted hover:text-foreground"
					>
						<LogIn className="size-4" />
						{t("Logowanie agentów", "Agents' sign-in")}
					</Link>
				</div>
			</header>
			<main className="mx-auto w-full max-w-4xl px-4 py-8 sm:px-8">
				{sent ? (
					<Card>
						<Card.Content className="flex flex-col items-center gap-4 py-12 text-center">
							<CheckCircle2 className="size-10 text-success" />
							<p className="text-sm text-muted">{t("Numer zgłoszenia", "Ticket number")}</p>
							<p className="font-mono text-3xl font-semibold" data-testid="ticket-number">
								{sent.number}
							</p>
							<p className="text-sm">
								{t("Termin realizacji", "Deadline")}:{" "}
								<strong>{formatInstant(sent.deadline, i18n)}</strong>
							</p>
							<Button
								variant="secondary"
								onPress={() => {
									setSent(null);
									report.reset();
									setRound((r) => r + 1);
								}}
							>
								{t("Zgłoś kolejną sprawę", "Report another request")}
							</Button>
						</Card.Content>
					</Card>
				) : (
					<Card>
						<Card.Content className="p-2 sm:p-4">
							{catalog.isPending || people.isPending ? (
								<Skeleton className="h-96 w-full rounded-xl" />
							) : catalog.isError || people.isError ? (
								<ApiError error={catalog.error ?? people.error} />
							) : (
								<ReportForm
									key={round}
									mode="public"
									catalog={catalog.data}
									people={people.data}
									isPending={report.isPending}
									error={report.error}
									onSubmit={async (v, files, fields) => {
										// The first write needs the XSRF-TOKEN cookie; this request sets it.
										await getCsrfToken();
										report.mutate(
											{
												body: {
													title: v.title.trim(),
													description: v.description.trim(),
													categoryKey: v.categoryKey,
													subcategoryKey: v.subcategoryKey,
													priority: v.priority,
													requesterPersonId: v.requesterPersonId,
													requesterIncognito: v.requesterIncognito,
													fields: JSON.stringify(fields),
												},
												files,
											},
											{ onSuccess: setSent },
										);
									}}
								/>
							)}
						</Card.Content>
					</Card>
				)}
			</main>
		</div>
	);
}
