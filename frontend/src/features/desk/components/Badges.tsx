import { Chip } from "@heroui/react";
import { AlarmClock, CheckCircle2, Clock } from "lucide-react";
import { useI18n } from "#/i18n";
import {
	deadlineState,
	formatInstant,
	type Priority,
	priorityColor,
	priorityLabel,
	statusLabel,
	timeLeftText,
} from "../types";

export function PriorityChip({ priority }: { priority: string }) {
	const i18n = useI18n();
	return (
		<Chip
			size="sm"
			variant="soft"
			className="whitespace-nowrap"
			color={priorityColor[priority as Priority] ?? "default"}
		>
			{priorityLabel(priority, i18n)}
		</Chip>
	);
}

export function StatusChip({ status }: { status: string }) {
	const i18n = useI18n();
	const color = status === "DONE" ? "success" : status === "IN_PROGRESS" ? "accent" : "default";
	return (
		<Chip size="sm" variant="soft" className="whitespace-nowrap" color={color}>
			{statusLabel(status, i18n)}
		</Chip>
	);
}

/**
 * Time left until the deadline: on time / due soon (warning) / overdue (alert) / resolved, on time or late.
 * Colour is never the only signal: each state has its own icon and words (ADR-0007).
 */
export function TimeLeft({
	status,
	deadline,
	resolvedLate,
	now,
}: {
	status: string;
	deadline: string;
	resolvedLate: boolean;
	now: number;
}) {
	const i18n = useI18n();
	const { t } = i18n;
	const state = deadlineState(status, deadline, now);
	const title = `${t("Termin", "Deadline")}: ${formatInstant(deadline, i18n)}`;
	if (state === "NONE")
		return (
			<span
				className={`inline-flex items-center gap-1 text-xs ${resolvedLate ? "text-danger" : "text-success"}`}
				data-state={resolvedLate ? "done-late" : "done"}
			>
				<CheckCircle2 className="size-3.5" />
				{resolvedLate
					? t("Zrealizowane po terminie", "Resolved late")
					: t("Zrealizowane", "Resolved")}
			</span>
		);
	const text = timeLeftText(deadline, now, i18n);
	if (state === "OVERDUE")
		return (
			<span
				className="inline-flex items-center gap-1 text-xs font-semibold text-danger"
				title={title}
				data-state="overdue"
			>
				<AlarmClock className="size-3.5" />
				{text}
			</span>
		);
	if (state === "DUE_SOON")
		return (
			<span
				className="inline-flex items-center gap-1 text-xs font-semibold text-warning"
				title={title}
				data-state="duesoon"
			>
				<Clock className="size-3.5" />
				{t(`zostało ${text}`, `${text} left`)}
			</span>
		);
	return (
		<span className="figures text-xs text-foreground" title={title} data-state="ontime">
			{text}
		</span>
	);
}
