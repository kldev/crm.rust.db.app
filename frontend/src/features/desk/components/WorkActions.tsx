import { Button, toast } from "@heroui/react";
import { ArrowLeftRight, CheckCheck, Hand, Undo2 } from "lucide-react";
import type { TicketResponse } from "#/api/models";
import { useMe } from "#/features/auth/session";
import { useI18n } from "#/i18n";
import { problemText } from "#/i18n/problems";
import { useResolveTicket, useTakeTicket } from "../hooks";
import type { TicketDialogsCommand } from "./TicketDialogs";

/**
 * What the signed-in agent can do with a ticket now: take it from the pool, or - when it is theirs - resolve it,
 * hand it over or give it back. Buttons mirror the rules only to hide what cannot work; the procedures decide, and
 * a refusal (someone was faster) comes back as a toast.
 */
export function WorkActions({
	ticket,
	dialogs,
	size = "sm",
}: {
	ticket: TicketResponse;
	dialogs: React.RefObject<TicketDialogsCommand | null>;
	size?: "sm" | "md";
}) {
	const i18n = useI18n();
	const { t } = i18n;
	const me = useMe();
	const onError = (error: unknown) => toast.danger(problemText(error, i18n));
	const take = useTakeTicket((x) => toast.success(t(`Przejęto ${x.number}`, `Took ${x.number}`)));
	const resolve = useResolveTicket((x) =>
		toast.success(t(`Zrealizowano ${x.number}`, `Resolved ${x.number}`)),
	);

	if (ticket.status === "IN_POOL")
		return (
			<Button
				size={size}
				variant="secondary"
				isPending={take.isPending}
				onPress={() => take.mutate(ticket.id, { onError })}
			>
				<Hand className="size-4" />
				{t("Przejmij", "Take")}
			</Button>
		);
	if (ticket.status !== "IN_PROGRESS" || ticket.assigneeUserId !== me.id) return null;
	return (
		<div className="inline-flex flex-wrap justify-end gap-2">
			<Button
				size={size}
				isPending={resolve.isPending}
				onPress={() => resolve.mutate(ticket.id, { onError })}
			>
				<CheckCheck className="size-4" />
				{t("Zrealizuj", "Resolve")}
			</Button>
			<Button size={size} variant="secondary" onPress={() => dialogs.current?.handOver(ticket)}>
				<ArrowLeftRight className="size-4" />
				{t("Przekaż", "Hand over")}
			</Button>
			<Button size={size} variant="ghost" onPress={() => dialogs.current?.returnToPool(ticket)}>
				<Undo2 className="size-4" />
				{t("Oddaj do puli", "Return")}
			</Button>
		</div>
	);
}
