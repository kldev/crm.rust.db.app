import { Button, Card, Modal, Skeleton } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Megaphone, Pencil } from "lucide-react";
import { useState } from "react";
import type { FieldDefinitionResponse, TicketResponse } from "#/api/models";
import { ApiError, InfoList } from "#/components/ui";
import { useI18n } from "#/i18n";
import { deskCatalogQuery, linkedQuery, ticketFieldsQuery, useUpdateFields } from "../hooks";
import { StatusChip } from "./Badges";
import { FieldInputs, type FieldValues } from "./FieldInputs";

/**
 * The ticket's report data (ADR-0010): its values, and a dialog to fill them in or correct them - any agent, in any
 * state, because numbers often arrive after the work is done. Hidden for subcategories without report data.
 */
export function TicketReportData({ ticket }: { ticket: TicketResponse }) {
	const i18n = useI18n();
	const { t, n } = i18n;
	const catalog = useQuery(deskCatalogQuery());
	const values = useQuery(ticketFieldsQuery(ticket.id));
	const [editing, setEditing] = useState(false);
	const definitions =
		catalog.data?.categories
			.flatMap((c) => c.subcategories)
			.find((s) => s.key === ticket.subcategoryKey)?.fields ?? [];
	if (definitions.length === 0) return null;
	const current = Object.fromEntries((values.data ?? []).map((v) => [v.key, v.value]));

	return (
		<Card>
			<Card.Header className="flex-row items-start justify-between gap-3">
				<div>
					<Card.Title>{t("Dane do raportu", "Report data")}</Card.Title>
					<Card.Description>
						{t("Trafiają do raportów i eksportów.", "They go into the reports and exports.")}
					</Card.Description>
				</div>
				<Button size="sm" variant="secondary" onPress={() => setEditing(true)}>
					<Pencil className="size-3.5" />
					{t("Uzupełnij", "Edit")}
				</Button>
			</Card.Header>
			<Card.Content>
				{values.isPending ? (
					<Skeleton className="h-16 w-full rounded-lg" />
				) : (
					<InfoList
						items={definitions.map((d) => ({
							label: n(d.label),
							value: shownValue(d, current[d.key], i18n),
						}))}
					/>
				)}
			</Card.Content>
			{editing ? (
				<EditDialog
					ticket={ticket}
					definitions={definitions}
					current={current}
					onClose={() => setEditing(false)}
				/>
			) : null}
		</Card>
	);
}

function shownValue(
	d: FieldDefinitionResponse,
	value: string | undefined,
	{ n }: { n: (v: string) => string },
) {
	if (!value) return "—";
	if (d.kind === "CHOICE") return n(value);
	if (d.kind === "MULTI_CHOICE") return value.split(", ").map(n).join(", ");
	if (d.kind === "TICKET_NUMBER") return <span className="font-mono text-xs">{value}</span>;
	return value;
}

function EditDialog({
	ticket,
	definitions,
	current,
	onClose,
}: {
	ticket: TicketResponse;
	definitions: FieldDefinitionResponse[];
	current: FieldValues;
	onClose: () => void;
}) {
	const { t } = useI18n();
	const [values, setValues] = useState<FieldValues>(current);
	const save = useUpdateFields(onClose);
	return (
		<Modal>
			<Modal.Backdrop isOpen onOpenChange={(open) => !open && onClose()}>
				<Modal.Container size="lg">
					<Modal.Dialog>
						<Modal.CloseTrigger />
						<Modal.Header>
							<Modal.Heading className="font-display text-2xl">
								{t("Dane do raportu", "Report data")}
							</Modal.Heading>
						</Modal.Header>
						<Modal.Body>
							<div className="flex flex-col gap-4">
								<FieldInputs fields={definitions} values={values} onChange={setValues} />
								<ApiError error={save.error} />
							</div>
						</Modal.Body>
						<Modal.Footer>
							<Button slot="close" variant="secondary">
								{t("Anuluj", "Cancel")}
							</Button>
							<Button
								isPending={save.isPending}
								onPress={() =>
									// The whole set: an emptied value clears the field.
									save.mutate({
										id: ticket.id,
										values: Object.fromEntries(
											definitions.map((d) => [d.key, values[d.key]?.trim() ?? ""]),
										),
									})
								}
							>
								{t("Zapisz", "Save")}
							</Button>
						</Modal.Footer>
					</Modal.Dialog>
				</Modal.Container>
			</Modal.Backdrop>
		</Modal>
	);
}

/** Recruitment: the campaigns ordered in marketing for this ticket, and a button that starts a new order. */
export function LinkedCampaigns({ ticket }: { ticket: TicketResponse }) {
	const { t } = useI18n();
	const linked = useQuery(linkedQuery(ticket.id));
	if (ticket.categoryKey !== "recruitment") return null;
	return (
		<Card>
			<Card.Header className="flex-row items-start justify-between gap-3">
				<div>
					<Card.Title>{t("Kampanie w marketingu", "Marketing campaigns")}</Card.Title>
					<Card.Description>
						{t("Zamówienia, które wskazują to zgłoszenie.", "Orders that point to this ticket.")}
					</Card.Description>
				</div>
				<Link
					to="/desk/new"
					search={{
						sub: "marketing.budget",
						title: t(
							`Kampania rekrutacyjna: ${ticket.title}`,
							`Recruitment campaign: ${ticket.title}`,
						),
						desc: t(
							`Kampania do zgłoszenia ${ticket.number}.`,
							`A campaign for ticket ${ticket.number}.`,
						),
						related: ticket.number,
					}}
				>
					<Button size="sm" variant="secondary">
						<Megaphone className="size-3.5" />
						{t("Zamów kampanię", "Order a campaign")}
					</Button>
				</Link>
			</Card.Header>
			<Card.Content>
				{linked.isPending ? (
					<Skeleton className="h-10 w-full rounded-lg" />
				) : linked.isError ? (
					<ApiError error={linked.error} />
				) : linked.data.length === 0 ? (
					<p className="text-sm text-muted">
						{t("Brak zamówionych kampanii.", "No campaigns ordered.")}
					</p>
				) : (
					<ul className="flex flex-col gap-2">
						{linked.data.map((c) => (
							<li key={c.id} className="flex items-center justify-between gap-3 text-sm">
								<Link
									to="/desk/tickets/$ticketId"
									params={{ ticketId: c.id }}
									className="hover:underline"
								>
									<span className="font-mono text-xs text-muted">{c.number}</span> {c.title}
								</Link>
								<StatusChip status={c.status} />
							</li>
						))}
					</ul>
				)}
			</Card.Content>
		</Card>
	);
}
