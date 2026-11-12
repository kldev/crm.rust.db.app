import {
	Banknote,
	Briefcase,
	Globe,
	IdCard,
	type LucideIcon,
	Megaphone,
	Monitor,
	Scale,
	Tag,
	UsersRound,
} from "lucide-react";

const ICONS: Record<string, LucideIcon> = {
	it: Monitor,
	legal: Scale,
	marketing: Megaphone,
	"hr-abroad": Globe,
	"hr-poland": Briefcase,
	recruitment: UsersRound,
	finance: Banknote,
	legalization: IdCard,
};

/** The icon of a category tile (as in Halcyon); an unknown key gets a plain tag. */
export function CategoryIcon({ categoryKey }: { categoryKey: string }) {
	const Icon = ICONS[categoryKey] ?? Tag;
	return (
		<span className="flex size-9 items-center justify-center rounded-lg bg-accent-soft text-ink">
			<Icon className="size-4" strokeWidth={1.75} />
		</span>
	);
}
