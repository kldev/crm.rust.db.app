import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { AlarmClock, Clock, Inbox, ListTodo, type LucideIcon } from "lucide-react";
import { useI18n } from "#/i18n";
import { countersQuery } from "../hooks";

/**
 * The desk's counters in the top bar: overdue and due soon link to the filtered list, then the pool and my tickets.
 * Refreshed every 30 s and after every desk action (the action invalidates the desk area).
 */
export function DeskCounters() {
	const { t } = useI18n();
	const counters = useQuery(countersQuery());
	const c = counters.data;
	if (!c) return null;
	return (
		<nav
			aria-label={t("Liczniki zgłoszeń", "Ticket counters")}
			className="hidden items-center gap-1 md:flex"
		>
			<Counter
				to="/desk/all"
				search={{ deadline: "OVERDUE" }}
				icon={AlarmClock}
				value={c.overdue}
				label={t("po terminie", "overdue")}
				tone={c.overdue > 0 ? "text-danger" : "text-muted"}
			/>
			<Counter
				to="/desk/all"
				search={{ deadline: "DUE_SOON" }}
				icon={Clock}
				value={c.dueSoon}
				label={t("kończy się", "due soon")}
				tone={c.dueSoon > 0 ? "text-warning" : "text-muted"}
			/>
			<Counter
				to="/desk/pool"
				icon={Inbox}
				value={c.pool}
				label={t("w puli", "in the pool")}
				tone="text-muted"
			/>
			<Counter
				to="/desk/mine"
				icon={ListTodo}
				value={c.mine}
				label={t("moje", "mine")}
				tone="text-muted"
			/>
		</nav>
	);
}

function Counter({
	to,
	search,
	icon: Icon,
	value,
	label,
	tone,
}: {
	to: "/desk/all" | "/desk/pool" | "/desk/mine";
	search?: { deadline: "OVERDUE" | "DUE_SOON" };
	icon: LucideIcon;
	value: number;
	label: string;
	tone: string;
}) {
	return (
		<Link
			to={to}
			search={search}
			className={`inline-flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs hover:bg-surface-secondary ${tone}`}
		>
			<Icon className="size-3.5" />
			<span className="figures font-semibold">{value}</span>
			<span>{label}</span>
		</Link>
	);
}
