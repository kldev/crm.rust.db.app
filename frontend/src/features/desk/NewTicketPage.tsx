import { Card, Skeleton, toast } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError } from "#/components/ui";
import { useI18n } from "#/i18n";
import { problemText } from "#/i18n/problems";
import type { FieldValues } from "./components/FieldInputs";
import { ReportForm, type ReportValues } from "./components/ReportForm";
import { deskCatalogQuery, peopleQuery, useAttachFiles, useReportTicket } from "./hooks";

const route = getRouteApi("/_app/desk/new");

/**
 * An agent registers a ticket taken by phone or e-mail; it may be dated back up to 30 days. Files go right after the
 * report: the ticket exists by then, so a refused file is reported on the ticket's page and nothing is lost.
 */
export function NewTicketPage() {
	const prefill = route.useSearch();
	const i18n = useI18n();
	const { t } = i18n;
	const navigate = useNavigate();
	const catalog = useQuery(deskCatalogQuery());
	const people = useQuery(peopleQuery());
	const report = useReportTicket();
	const attach = useAttachFiles();

	const submit = (v: ReportValues, files: File[], fields: FieldValues) =>
		report.mutate(
			{
				title: v.title.trim(),
				description: v.description.trim(),
				categoryKey: v.categoryKey,
				subcategoryKey: v.subcategoryKey,
				priority: v.priority,
				// datetime-local is the browser's time; Date turns it into the instant.
				reportedAt: new Date(v.reportedAt).toISOString(),
				requesterPersonId: v.requesterPersonId,
				requesterIncognito: v.requesterIncognito,
				fields,
			},
			{
				onSuccess: async (ticket) => {
					if (files.length > 0) {
						try {
							await attach.mutateAsync({ id: ticket.id, files });
						} catch (error) {
							toast.danger(
								t(
									`Zgłoszenie ${ticket.number} zapisane, ale pliki nie: ${problemText(error, i18n)}`,
									`Ticket ${ticket.number} saved, but not the files: ${problemText(error, i18n)}`,
								),
							);
						}
					}
					toast.success(t(`Zarejestrowano ${ticket.number}`, `Registered ${ticket.number}`));
					void navigate({ to: "/desk/tickets/$ticketId", params: { ticketId: ticket.id } });
				},
			},
		);

	return (
		<>
			<PageHeader
				eyebrow={t("Service desk", "Service desk")}
				title={t("Nowe zgłoszenie", "New ticket")}
				description={t(
					"Sprawa zgłoszona telefonicznie albo mailem. Trafi do puli.",
					"A request made by phone or e-mail. It goes to the pool.",
				)}
			/>
			<Card>
				<Card.Content className="p-2 sm:p-4">
					{catalog.isPending || people.isPending ? (
						<Skeleton className="h-96 w-full rounded-xl" />
					) : catalog.isError || people.isError ? (
						<ApiError error={catalog.error ?? people.error} />
					) : (
						<ReportForm
							mode="agent"
							catalog={catalog.data}
							people={people.data}
							isPending={report.isPending || attach.isPending}
							error={report.error}
							onSubmit={submit}
							initial={{
								categoryKey: catalog.data.categories.find((c) =>
									c.subcategories.some((s) => s.key === prefill.sub),
								)?.key,
								subcategoryKey: prefill.sub,
								title: prefill.title,
								description: prefill.desc,
								fields: prefill.related ? { "related-ticket": prefill.related } : undefined,
							}}
						/>
					)}
				</Card.Content>
			</Card>
		</>
	);
}
