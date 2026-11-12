import {
	ArrowLeftRight,
	CheckCheck,
	CirclePlus,
	Gauge,
	Hand,
	type LucideIcon,
	MessageSquare,
	Paperclip,
	TableProperties,
	Undo2,
	UserRoundCog,
} from "lucide-react";
import { useI18n } from "#/i18n";
import { describeEvent, formatInstant, type TimelineEvent } from "../types";

const ICONS: Record<string, LucideIcon> = {
	TicketReported: CirclePlus,
	TicketTaken: Hand,
	TicketHandedOver: ArrowLeftRight,
	TicketReturnedToPool: Undo2,
	TicketPriorityChanged: Gauge,
	TicketResolved: CheckCheck,
	TicketCommented: MessageSquare,
	TicketHandlerAssigned: UserRoundCog,
	TicketAttachmentAdded: Paperclip,
	TicketFieldsUpdated: TableProperties,
};

/** The ticket's stream as a timeline. Free text (comments, reasons) is shown as text, never as markup. */
export function Timeline({ events }: { events: TimelineEvent[] }) {
	const i18n = useI18n();
	return (
		<ol className="flex flex-col gap-4">
			{events.map((e) => {
				const { title, who, text } = describeEvent(e, i18n);
				const Icon = ICONS[e.type] ?? CirclePlus;
				return (
					<li key={e.version} className="flex gap-3" data-event={e.type}>
						<span className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-full bg-accent-soft text-ink">
							<Icon className="size-3.5" strokeWidth={1.75} />
						</span>
						<div className="min-w-0 flex-1">
							<p className="text-sm text-foreground">{title}</p>
							<p className="text-xs text-muted">
								{who ? `${who} · ` : ""}
								{formatInstant(e.at, i18n)}
							</p>
							{text ? (
								<p className="mt-1 rounded-md bg-surface-secondary px-3 py-2 text-sm whitespace-pre-wrap text-foreground">
									{text}
								</p>
							) : null}
						</div>
					</li>
				);
			})}
		</ol>
	);
}
