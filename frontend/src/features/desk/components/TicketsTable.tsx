import { Table } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import type { ReactNode } from "react";
import type { DeskCatalog, TicketResponse } from "#/api/models";
import { useI18n } from "#/i18n";
import { deadlineState } from "../types";
import { PriorityChip, StatusChip, TimeLeft } from "./Badges";
import { useCatalogNames } from "./useCatalogNames";

/**
 * Tickets in list order; a row opens the ticket. Overdue rows are tinted red, due soon amber. `actions` renders the
 * buttons of a row (Take in the pool, the work actions on my tickets); the page owns what they do.
 */
export function TicketsTable({
	tickets,
	catalog,
	now,
	actions,
}: {
	tickets: TicketResponse[];
	catalog: DeskCatalog | undefined;
	now: number;
	actions?: (ticket: TicketResponse) => ReactNode;
}) {
	const i18n = useI18n();
	const { t } = i18n;
	const navigate = useNavigate();
	const names = useCatalogNames(catalog);
	return (
		<Table className="rounded-none shadow-none">
			<Table.ScrollContainer>
				<Table.Content aria-label={t("Zgłoszenia", "Tickets")} className="min-w-[860px]">
					<Table.Header>
						<Table.Column isRowHeader>{t("Zgłoszenie", "Ticket")}</Table.Column>
						<Table.Column>{t("Kategoria", "Category")}</Table.Column>
						<Table.Column>{t("Priorytet", "Priority")}</Table.Column>
						<Table.Column>{t("Status", "Status")}</Table.Column>
						<Table.Column>{t("Termin", "Deadline")}</Table.Column>
						<Table.Column className="text-right">
							<span className="sr-only">{t("Akcje", "Actions")}</span>
						</Table.Column>
					</Table.Header>
					<Table.Body>
						{tickets.map((ticket) => {
							const state = deadlineState(ticket.status, ticket.deadline, now);
							const tint =
								state === "OVERDUE"
									? "bg-danger-soft/40"
									: state === "DUE_SOON"
										? "bg-warning-soft/40"
										: "";
							return (
								<Table.Row
									key={ticket.id}
									id={ticket.id}
									className={`is-clickable ${tint}`}
									onAction={() =>
										void navigate({
											to: "/desk/tickets/$ticketId",
											params: { ticketId: ticket.id },
										})
									}
								>
									<Table.Cell>
										<div className="flex flex-col">
											<span className="font-medium text-foreground">{ticket.title}</span>
											<span className="font-mono text-[11px] text-muted">
												{ticket.number}
												{ticket.requesterIncognito
													? " · incognito"
													: ticket.requesterName
														? ` · ${ticket.requesterName}`
														: ""}
											</span>
										</div>
									</Table.Cell>
									<Table.Cell>
										<div className="flex flex-col">
											<span>{names.category(ticket.categoryKey)}</span>
											<span className="text-xs text-muted">
												{names.subcategory(ticket.subcategoryKey)}
											</span>
										</div>
									</Table.Cell>
									<Table.Cell>
										<PriorityChip priority={ticket.priority} />
									</Table.Cell>
									<Table.Cell>
										<div className="flex flex-col items-start gap-1">
											<StatusChip status={ticket.status} />
											{ticket.assigneeName ? (
												<span className="text-xs text-muted">{ticket.assigneeName}</span>
											) : null}
										</div>
									</Table.Cell>
									<Table.Cell>
										<TimeLeft
											status={ticket.status}
											deadline={ticket.deadline}
											resolvedLate={ticket.resolvedLate}
											now={now}
										/>
									</Table.Cell>
									<Table.Cell className="text-right">{actions?.(ticket)}</Table.Cell>
								</Table.Row>
							);
						})}
					</Table.Body>
				</Table.Content>
			</Table.ScrollContainer>
		</Table>
	);
}
