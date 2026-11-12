import { Chip } from "@heroui/react";
import { type ClientStatus, clientStatusColor, clientStatusLabel } from "#/features/types";

export function ClientStatusChip({ status }: { status: string }) {
	const s = status as ClientStatus;
	return (
		<Chip size="sm" variant="soft" color={clientStatusColor[s] ?? "default"}>
			{clientStatusLabel[s] ?? status}
		</Chip>
	);
}
