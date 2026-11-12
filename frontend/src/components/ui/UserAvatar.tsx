import { Avatar } from "@heroui/react";

/** Circuit avatars: navy disc, cyan initials. */
export function UserAvatar({ name, size = "sm" }: { name: string; size?: "sm" | "md" | "lg" }) {
	const initials = name
		.split(/[\s.]+/)
		.filter(Boolean)
		.slice(0, 2)
		.map((part) => part[0]?.toUpperCase())
		.join("");
	return (
		<Avatar size={size} className="bg-ink text-signal">
			<Avatar.Fallback className="bg-ink font-mono text-[11px] font-semibold tracking-wider text-signal">
				{initials || "?"}
			</Avatar.Fallback>
		</Avatar>
	);
}
