import { Chip } from "@heroui/react";
import { type Stage, stageColor, stageLabel } from "#/features/types";

export function StageChip({ stage, size = "sm" }: { stage: string; size?: "sm" | "md" | "lg" }) {
	const s = stage as Stage;
	return (
		<Chip size={size} variant="soft" color={stageColor[s] ?? "default"}>
			{stageLabel[s] ?? stage}
		</Chip>
	);
}

export function DueDate({ date, overdue }: { date: string; overdue: boolean }) {
	return (
		<span
			className={`figures text-xs ${overdue ? "font-semibold text-danger" : "text-foreground"}`}
		>
			{date}
			{overdue ? " · overdue" : ""}
		</span>
	);
}
