import { Button, Card, Label, Skeleton, TextArea, TextField, toast } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { getRouteApi, Link } from "@tanstack/react-router";
import { ArrowLeft, Gauge, UserRoundCog } from "lucide-react";
import { useRef, useState } from "react";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, InfoList } from "#/components/ui";
import { useMe } from "#/features/auth/session";
import { useNow } from "#/hooks/useNow";
import { useI18n } from "#/i18n";
import { PriorityChip, StatusChip, TimeLeft } from "./components/Badges";
import { TicketAttachments } from "./components/TicketAttachments";
import { TicketDialogs, type TicketDialogsCommand } from "./components/TicketDialogs";
import { LinkedCampaigns, TicketReportData } from "./components/TicketReportData";
import { Timeline } from "./components/Timeline";
import { useCatalogNames } from "./components/useCatalogNames";
import { WorkActions } from "./components/WorkActions";
import { deskCatalogQuery, ticketEventsQuery, ticketQuery, useCommentTicket } from "./hooks";
import { formatInstant, priorityLabel } from "./types";

const route = getRouteApi("/_app/desk/tickets/$ticketId");

export function TicketDetailsPage() {
	const { ticketId } = route.useParams();
	const i18n = useI18n();
	const { t } = i18n;
	const me = useMe();
	const now = useNow();
	const dialogs = useRef<TicketDialogsCommand>(null);
	const ticket = useQuery(ticketQuery(ticketId));
	const events = useQuery(ticketEventsQuery(ticketId));
	const catalog = useQuery(deskCatalogQuery());
	const names = useCatalogNames(catalog.data);
	const [comment, setComment] = useState("");
	const addComment = useCommentTicket(() => {
		setComment("");
		toast.success(t("Dodano komentarz", "Comment added"));
	});

	const back = (
		<Link to="/desk/pool" className="inline-flex items-center gap-1 hover:text-foreground">
			<ArrowLeft className="size-3" /> {t("Pula", "Pool")}
		</Link>
	);
	if (ticket.isPending)
		return (
			<div className="flex flex-col gap-4">
				<Skeleton className="h-10 w-80 rounded-lg" />
				<Skeleton className="h-64 w-full rounded-xl" />
			</div>
		);
	if (ticket.isError)
		return (
			<>
				<PageHeader eyebrow={back} title={t("Zgłoszenie", "Ticket")} />
				<ApiError
					error={ticket.error}
					title={t("Nie udało się wczytać zgłoszenia", "The ticket could not be loaded")}
				/>
			</>
		);

	const x = ticket.data;
	const isMine = x.status === "IN_PROGRESS" && x.assigneeUserId === me.id;
	return (
		<>
			<PageHeader
				eyebrow={back}
				title={x.title}
				meta={
					<>
						<span className="font-mono text-xs text-muted">{x.number}</span>
						<StatusChip status={x.status} />
						<PriorityChip priority={x.priority} />
						<TimeLeft
							status={x.status}
							deadline={x.deadline}
							resolvedLate={x.resolvedLate}
							now={now}
						/>
					</>
				}
				actions={
					<>
						<WorkActions ticket={x} dialogs={dialogs} size="md" />
						{x.status !== "DONE" ? (
							<Button variant="secondary" onPress={() => dialogs.current?.changePriority(x)}>
								<Gauge className="size-4" />
								{t("Priorytet", "Priority")}
							</Button>
						) : null}
						{isMine ? (
							<Button variant="secondary" onPress={() => dialogs.current?.assignHandler(x)}>
								<UserRoundCog className="size-4" />
								{t("Osoba realizująca", "Handler")}
							</Button>
						) : null}
					</>
				}
			/>

			<div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_340px]">
				<div className="flex flex-col gap-6">
					<Card>
						<Card.Header>
							<Card.Title>{t("Opis", "Description")}</Card.Title>
						</Card.Header>
						<Card.Content>
							<p className="text-sm whitespace-pre-wrap text-foreground">{x.description}</p>
						</Card.Content>
					</Card>

					<TicketReportData ticket={x} />

					<LinkedCampaigns ticket={x} />

					<TicketAttachments ticket={x} />

					<Card>
						<Card.Header>
							<Card.Title>{t("Komentarz", "Comment")}</Card.Title>
							<Card.Description>
								{t(
									"Komentarzy nie można zmienić ani usunąć - są częścią historii zgłoszenia.",
									"Comments cannot be changed or deleted - they are part of the ticket's history.",
								)}
							</Card.Description>
						</Card.Header>
						<Card.Content className="flex flex-col gap-3">
							<TextField value={comment} onChange={setComment} fullWidth>
								<Label className="sr-only">{t("Komentarz", "Comment")}</Label>
								<TextArea
									rows={3}
									maxLength={2000}
									placeholder={t("Co zrobiono, co dalej…", "What was done, what next…")}
								/>
							</TextField>
							<ApiError error={addComment.error} />
							<div className="flex justify-end">
								<Button
									isDisabled={comment.trim() === ""}
									isPending={addComment.isPending}
									onPress={() => addComment.mutate({ id: x.id, text: comment.trim() })}
								>
									{t("Dodaj komentarz", "Add comment")}
								</Button>
							</div>
						</Card.Content>
					</Card>

					<Card>
						<Card.Header>
							<Card.Title>{t("Historia", "History")}</Card.Title>
							<Card.Description>
								{t("Zdarzenia zgłoszenia, od najstarszego.", "The ticket's events, oldest first.")}
							</Card.Description>
						</Card.Header>
						<Card.Content>
							{events.isPending ? (
								<Skeleton className="h-24 w-full rounded-lg" />
							) : events.isError ? (
								<ApiError error={events.error} />
							) : (
								<Timeline events={events.data} />
							)}
						</Card.Content>
					</Card>
				</div>

				<Card className="h-fit">
					<Card.Content>
						<InfoList
							items={[
								{ label: t("Kategoria", "Category"), value: names.category(x.categoryKey) },
								{
									label: t("Podkategoria", "Subcategory"),
									value: names.subcategory(x.subcategoryKey),
								},
								{
									label: t("Zgłaszający", "Requester"),
									value: x.requesterIncognito ? "incognito" : (x.requesterName ?? "—"),
								},
								{
									label: t("Kanał", "Channel"),
									value:
										x.channel === "SELF_SERVICE"
											? t("Formularz publiczny", "Public form")
											: t(
													`Agent: ${x.registeredByName ?? ""}`,
													`Agent: ${x.registeredByName ?? ""}`,
												),
								},
								{ label: t("Zgłoszono", "Reported"), value: formatInstant(x.reportedAt, i18n) },
								{ label: t("Termin", "Deadline"), value: formatInstant(x.deadline, i18n) },
								{
									label: t("Priorytet początkowy", "Original priority"),
									value: priorityLabel(x.originalPriority, i18n),
								},
								{
									label:
										x.status === "DONE" ? t("Zrealizował(a)", "Resolved by") : t("Agent", "Agent"),
									value: x.assigneeName ?? "—",
								},
								{ label: t("Osoba realizująca", "Handler"), value: x.handlerName ?? "—" },
								...(x.resolvedAt
									? [
											{
												label: t("Zrealizowano", "Resolved"),
												value: formatInstant(x.resolvedAt, i18n),
											},
										]
									: []),
							]}
						/>
					</Card.Content>
				</Card>
			</div>

			<TicketDialogs ref={dialogs} />
		</>
	);
}
