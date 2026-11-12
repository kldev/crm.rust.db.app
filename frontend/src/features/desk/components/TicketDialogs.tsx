import {
	Button,
	Description,
	Label,
	ListBox,
	Modal,
	SearchField,
	Select,
	TextArea,
	TextField,
} from "@heroui/react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { forwardRef, type ReactNode, useImperativeHandle, useMemo, useState } from "react";
import type { TicketResponse } from "#/api/models";
import { ApiError } from "#/components/ui";
import { useMe } from "#/features/auth/session";
import { useDebouncedValue } from "#/hooks/useDebouncedValue";
import { useI18n } from "#/i18n";
import {
	agentsQuery,
	deskCatalogQuery,
	peopleQuery,
	useAssignHandler,
	useChangePriority,
	useHandOverTicket,
	useReturnTicket,
} from "../hooks";
import { formatInstant, PRIORITIES, priorityLabel, slaLabel } from "../types";

/** Commands of the ticket dialogs. The page owns one instance, so a ticket is never open in two dialogs. */
export interface TicketDialogsCommand {
	handOver: (ticket: TicketResponse) => void;
	returnToPool: (ticket: TicketResponse) => void;
	changePriority: (ticket: TicketResponse) => void;
	assignHandler: (ticket: TicketResponse) => void;
}

type Open = { kind: keyof TicketDialogsCommand; ticket: TicketResponse };

export const TicketDialogs = forwardRef<
	TicketDialogsCommand,
	{ onDone?: (t: TicketResponse) => void }
>(({ onDone }, ref) => {
	const [open, setOpen] = useState<Open | null>(null);
	useImperativeHandle(
		ref,
		() => ({
			handOver: (ticket) => setOpen({ kind: "handOver", ticket }),
			returnToPool: (ticket) => setOpen({ kind: "returnToPool", ticket }),
			changePriority: (ticket) => setOpen({ kind: "changePriority", ticket }),
			assignHandler: (ticket) => setOpen({ kind: "assignHandler", ticket }),
		}),
		[],
	);
	if (!open) return null;
	const close = () => setOpen(null);
	const done = (t: TicketResponse) => {
		setOpen(null);
		onDone?.(t);
	};
	const props = { ticket: open.ticket, onClose: close, onDone: done };
	switch (open.kind) {
		case "handOver":
			return <HandOverDialog {...props} />;
		case "returnToPool":
			return <ReturnDialog {...props} />;
		case "changePriority":
			return <PriorityDialog {...props} />;
		case "assignHandler":
			return <HandlerDialog {...props} />;
	}
});

interface DialogProps {
	ticket: TicketResponse;
	onClose: () => void;
	onDone: (t: TicketResponse) => void;
}

function ActionModal({
	title,
	subtitle,
	onClose,
	children,
	submitLabel,
	onSubmit,
	isPending,
	isDisabled,
	error,
}: {
	title: string;
	subtitle: string;
	onClose: () => void;
	children: ReactNode;
	submitLabel: string;
	onSubmit: () => void;
	isPending: boolean;
	isDisabled?: boolean;
	error: unknown;
}) {
	const { t } = useI18n();
	return (
		<Modal>
			<Modal.Backdrop isOpen onOpenChange={(isOpen) => !isOpen && onClose()}>
				<Modal.Container size="md">
					<Modal.Dialog>
						<Modal.CloseTrigger />
						<Modal.Header>
							<div className="flex flex-col gap-1">
								<Modal.Heading className="font-display text-2xl">{title}</Modal.Heading>
								<p className="text-sm text-muted">{subtitle}</p>
							</div>
						</Modal.Header>
						<Modal.Body>
							<div className="flex flex-col gap-4">
								{children}
								<ApiError error={error} />
							</div>
						</Modal.Body>
						<Modal.Footer>
							<Button slot="close" variant="secondary">
								{t("Anuluj", "Cancel")}
							</Button>
							<Button isPending={isPending} isDisabled={isDisabled} onPress={onSubmit}>
								{submitLabel}
							</Button>
						</Modal.Footer>
					</Modal.Dialog>
				</Modal.Container>
			</Modal.Backdrop>
		</Modal>
	);
}

function ReasonField({ value, onChange }: { value: string; onChange: (v: string) => void }) {
	const { t } = useI18n();
	return (
		<TextField value={value} onChange={onChange} fullWidth>
			<Label>{t("Powód (opcjonalnie)", "Reason (optional)")}</Label>
			<TextArea rows={3} maxLength={500} />
		</TextField>
	);
}

const subtitleOf = (ticket: TicketResponse) => `${ticket.number} · ${ticket.title}`;
const orNull = (v: string) => (v.trim() === "" ? null : v.trim());

function HandOverDialog({ ticket, onClose, onDone }: DialogProps) {
	const { t } = useI18n();
	const me = useMe();
	const [text, setText] = useState("");
	const search = useDebouncedValue(text.trim(), 300);
	const [toUserId, setToUserId] = useState<number | null>(null);
	const [reason, setReason] = useState("");
	const agents = useInfiniteQuery(agentsQuery(search || undefined, 20));
	const list = (agents.data?.pages.flatMap((p) => p.items) ?? []).filter((a) => a.id !== me.id);
	const mutation = useHandOverTicket(onDone);
	return (
		<ActionModal
			title={t("Przekaż zgłoszenie", "Hand the ticket over")}
			subtitle={subtitleOf(ticket)}
			onClose={onClose}
			submitLabel={t("Przekaż", "Hand over")}
			isPending={mutation.isPending}
			isDisabled={toUserId == null}
			error={mutation.error}
			onSubmit={() =>
				toUserId != null && mutation.mutate({ id: ticket.id, toUserId, reason: orNull(reason) })
			}
		>
			<SearchField
				aria-label={t("Szukaj agenta", "Search agents")}
				value={text}
				onChange={setText}
				fullWidth
			>
				<SearchField.Group>
					<SearchField.SearchIcon />
					<SearchField.Input placeholder={t("Imię, nazwisko albo login…", "Name or login…")} />
					<SearchField.ClearButton />
				</SearchField.Group>
			</SearchField>
			<ListBox
				aria-label={t("Agenci", "Agents")}
				selectionMode="single"
				className="max-h-64 overflow-y-auto rounded-md border border-separator"
				selectedKeys={toUserId != null ? new Set([toUserId]) : new Set()}
				onSelectionChange={(keys) => {
					const key = keys === "all" ? undefined : [...keys][0];
					setToUserId(key == null ? null : Number(key));
				}}
			>
				{list.map((a) => (
					<ListBox.Item key={a.id} id={a.id} textValue={a.displayName}>
						<div className="flex w-full items-center justify-between gap-3">
							<span className="flex flex-col">
								<span>{a.displayName}</span>
								<span className="font-mono text-[11px] text-muted">{a.login}</span>
							</span>
							<span className="text-xs text-muted">
								{t(`w realizacji: ${a.inProgress}`, `in progress: ${a.inProgress}`)}
							</span>
						</div>
						<ListBox.ItemIndicator />
					</ListBox.Item>
				))}
			</ListBox>
			<ReasonField value={reason} onChange={setReason} />
			<p className="text-xs text-muted">
				{t("Termin się nie zmienia.", "The deadline stays the same.")}
			</p>
		</ActionModal>
	);
}

function ReturnDialog({ ticket, onClose, onDone }: DialogProps) {
	const { t } = useI18n();
	const [reason, setReason] = useState("");
	const mutation = useReturnTicket(onDone);
	return (
		<ActionModal
			title={t("Oddaj do puli", "Return to the pool")}
			subtitle={subtitleOf(ticket)}
			onClose={onClose}
			submitLabel={t("Oddaj do puli", "Return")}
			isPending={mutation.isPending}
			error={mutation.error}
			onSubmit={() => mutation.mutate({ id: ticket.id, reason: orNull(reason) })}
		>
			<ReasonField value={reason} onChange={setReason} />
			<p className="text-xs text-muted">
				{t("Termin się nie zmienia.", "The deadline stays the same.")}
			</p>
		</ActionModal>
	);
}

function PriorityDialog({ ticket, onClose, onDone }: DialogProps) {
	const i18n = useI18n();
	const { t } = i18n;
	const catalog = useQuery(deskCatalogQuery());
	const [priority, setPriority] = useState(ticket.priority);
	const [reason, setReason] = useState("");
	const mutation = useChangePriority(onDone);
	// The new deadline counts from the REPORT date, as the procedure does - shown before saving.
	const sla = catalog.data?.priorities.find((p) => p.priority === priority)?.slaMinutes;
	const preview = sla ? new Date(Date.parse(ticket.reportedAt) + sla * 60_000).toISOString() : null;
	return (
		<ActionModal
			title={t("Zmień priorytet", "Change the priority")}
			subtitle={subtitleOf(ticket)}
			onClose={onClose}
			submitLabel={t("Zmień", "Change")}
			isPending={mutation.isPending}
			isDisabled={priority === ticket.priority}
			error={mutation.error}
			onSubmit={() => mutation.mutate({ id: ticket.id, priority, reason: orNull(reason) })}
		>
			<Select
				fullWidth
				value={priority}
				onChange={(key) => key != null && setPriority(String(key))}
			>
				<Label>{t("Priorytet", "Priority")}</Label>
				<Select.Trigger>
					<Select.Value />
					<Select.Indicator />
				</Select.Trigger>
				<Select.Popover>
					<ListBox>
						{PRIORITIES.map((p) => (
							<ListBox.Item key={p} id={p} textValue={priorityLabel(p, i18n)}>
								<span className="flex w-full justify-between gap-3">
									<span>{priorityLabel(p, i18n)}</span>
									<span className="text-xs text-muted">{slaLabel(p, i18n)}</span>
								</span>
								<ListBox.ItemIndicator />
							</ListBox.Item>
						))}
					</ListBox>
				</Select.Popover>
			</Select>
			{preview ? (
				<p className="text-sm">
					{t("Nowy termin", "New deadline")}:{" "}
					<strong className="figures">{formatInstant(preview, i18n)}</strong>{" "}
					<span className="text-xs text-muted">
						{t("(liczony od daty zgłoszenia)", "(counted from the report date)")}
					</span>
				</p>
			) : null}
			<ReasonField value={reason} onChange={setReason} />
		</ActionModal>
	);
}

const NOBODY = "__nobody";

function HandlerDialog({ ticket, onClose, onDone }: DialogProps) {
	const { t, n } = useI18n();
	const catalog = useQuery(deskCatalogQuery());
	const people = useQuery(peopleQuery());
	const department =
		catalog.data?.categories.find((c) => c.key === ticket.categoryKey)?.department ?? null;
	const [personId, setPersonId] = useState<string>(
		ticket.handlerPersonId ? String(ticket.handlerPersonId) : NOBODY,
	);
	const [reason, setReason] = useState("");
	const mutation = useAssignHandler(onDone);
	// Only people of the category's department, when it has one (the procedure refuses others, 51115).
	const options = useMemo(
		() => (people.data ?? []).filter((p) => !department || p.department === department),
		[people.data, department],
	);
	return (
		<ActionModal
			title={t("Osoba realizująca", "Person doing the work")}
			subtitle={subtitleOf(ticket)}
			onClose={onClose}
			submitLabel={t("Zapisz", "Save")}
			isPending={mutation.isPending}
			error={mutation.error}
			onSubmit={() =>
				mutation.mutate({
					id: ticket.id,
					personId: personId === NOBODY ? null : Number(personId),
					reason: orNull(reason),
				})
			}
		>
			<Select
				fullWidth
				value={personId}
				onChange={(key) => key != null && setPersonId(String(key))}
			>
				<Label>{t("Osoba", "Person")}</Label>
				<Select.Trigger>
					<Select.Value />
					<Select.Indicator />
				</Select.Trigger>
				{department ? (
					<Description>
						{t(
							`Tylko osoby z działu ${department}.`,
							`Only people from the ${n(department)} department.`,
						)}
					</Description>
				) : null}
				<Select.Popover>
					<ListBox>
						<ListBox.Item id={NOBODY} textValue={t("Nikt", "Nobody")}>
							{t("Bez osoby realizującej", "No person doing the work")}
							<ListBox.ItemIndicator />
						</ListBox.Item>
						{options.map((p) => (
							<ListBox.Item key={p.id} id={String(p.id)} textValue={`${p.firstName} ${p.lastName}`}>
								<span className="flex w-full justify-between gap-3">
									<span>
										{p.firstName} {p.lastName}
									</span>
									<span className="text-xs text-muted">{n(p.department)}</span>
								</span>
								<ListBox.ItemIndicator />
							</ListBox.Item>
						))}
					</ListBox>
				</Select.Popover>
			</Select>
			<ReasonField value={reason} onChange={setReason} />
		</ActionModal>
	);
}
