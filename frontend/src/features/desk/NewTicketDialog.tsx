import { Modal, Skeleton, toast } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { createContext, type ReactNode, useCallback, useContext, useMemo, useState } from "react";
import { ApiError } from "#/components/ui";
import { useI18n } from "#/i18n";
import { problemText } from "#/i18n/problems";
import type { FieldValues } from "./components/FieldInputs";
import { ReportForm, type ReportValues } from "./components/ReportForm";
import { deskCatalogQuery, peopleQuery, useAttachFiles, useReportTicket } from "./hooks";

/** What a link may fill: "order a campaign" from a recruitment ticket sets the topic, text and the related ticket. */
export interface NewTicketPrefill {
	sub?: string;
	title?: string;
	desc?: string;
	related?: string;
}

const NewTicketContext = createContext<((prefill?: NewTicketPrefill) => void) | null>(null);

/** Opens the "new ticket" dialog from anywhere in the signed-in app. */
export function useNewTicket() {
	const open = useContext(NewTicketContext);
	if (!open) throw new Error("useNewTicket outside NewTicketProvider");
	return open;
}

/** Holds the dialog for the signed-in app; each opening starts a fresh form. */
export function NewTicketProvider({ children }: { children: ReactNode }) {
	const [prefill, setPrefill] = useState<NewTicketPrefill | null>(null);
	const open = useCallback((p?: NewTicketPrefill) => setPrefill(p ?? {}), []);
	return (
		<NewTicketContext.Provider value={open}>
			{children}
			{prefill ? <NewTicketDialog prefill={prefill} onClose={() => setPrefill(null)} /> : null}
		</NewTicketContext.Provider>
	);
}

/**
 * An agent registers a ticket taken by phone or e-mail, step by step in a dialog; it may be dated back up to 30 days.
 * Files go right after the report: the ticket exists by then, so a refused file is reported on the ticket's page and
 * nothing is lost.
 */
function NewTicketDialog({ prefill, onClose }: { prefill: NewTicketPrefill; onClose: () => void }) {
	const i18n = useI18n();
	const { t } = i18n;
	const navigate = useNavigate();
	const catalog = useQuery(deskCatalogQuery());
	const people = useQuery(peopleQuery());
	const report = useReportTicket();
	const attach = useAttachFiles();

	const initial = useMemo(
		() =>
			catalog.data
				? {
						categoryKey: catalog.data.categories.find((c) =>
							c.subcategories.some((s) => s.key === prefill.sub),
						)?.key,
						subcategoryKey: prefill.sub,
						title: prefill.title,
						description: prefill.desc,
						fields: prefill.related ? { "related-ticket": prefill.related } : undefined,
					}
				: undefined,
		[catalog.data, prefill],
	);

	const submit = (v: ReportValues, files: File[], fields: FieldValues) =>
		report.mutate(
			{
				title: v.title.trim(),
				description: v.description.trim(),
				categoryKey: v.categoryKey,
				subcategoryKey: v.subcategoryKey,
				priority: v.priority,
				// The date picker gives the browser's local time; Date turns it into the instant.
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
					onClose();
					void navigate({ to: "/desk/tickets/$ticketId", params: { ticketId: ticket.id } });
				},
			},
		);

	return (
		<Modal>
			<Modal.Backdrop isOpen onOpenChange={(open) => !open && onClose()}>
				<Modal.Container placement="center" scroll="inside">
					<Modal.Dialog className="w-full sm:max-w-4xl">
						<Modal.CloseTrigger />
						<Modal.Header className="border-b border-separator pb-4">
							<div className="flex flex-col gap-1">
								<Modal.Heading className="font-display text-xl font-semibold">
									{t("Nowe zgłoszenie", "New ticket")}
								</Modal.Heading>
								<p className="text-sm text-muted">
									{t(
										"Sprawa zgłoszona telefonicznie albo mailem. Trafi do puli.",
										"A request made by phone or e-mail. It goes to the pool.",
									)}
								</p>
							</div>
						</Modal.Header>
						{catalog.isPending || people.isPending ? (
							<Modal.Body>
								<Skeleton className="h-96 w-full rounded-md" />
							</Modal.Body>
						) : catalog.isError || people.isError ? (
							<Modal.Body>
								<ApiError error={catalog.error ?? people.error} />
							</Modal.Body>
						) : (
							<ReportForm
								variant="wizard"
								mode="agent"
								catalog={catalog.data}
								people={people.data}
								isPending={report.isPending || attach.isPending}
								error={report.error}
								onSubmit={submit}
								onCancel={onClose}
								initial={initial}
							/>
						)}
					</Modal.Dialog>
				</Modal.Container>
			</Modal.Backdrop>
		</Modal>
	);
}
