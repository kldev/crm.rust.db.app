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
		<span className="flex size-10 items-center justify-center rounded-md bg-accent-soft text-accent">
			<Icon className="size-5" strokeWidth={1.75} />
		</span>
	);
}
