import { Card, Skeleton } from "@heroui/react";
import {
	type InfiniteData,
	type UseInfiniteQueryResult,
	useInfiniteQuery,
} from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import {
	AlarmClock,
	ArrowRight,
	BriefcaseBusiness,
	Building2,
	CalendarRange,
	UsersRound,
} from "lucide-react";
import type { ReactNode } from "react";
import type { PageResponseProjectResponse } from "#/api/models";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError } from "#/components/ui";
import { useMe } from "#/features/auth/session";
import { ClientStatusChip } from "#/features/clients/ClientStatusChip";
import { clientsQuery } from "#/features/clients/hooks";
import { projectsQuery } from "#/features/projects/hooks";
import { StageChip } from "#/features/projects/StageChip";
import { roleLabel } from "#/features/types";
import { daysUntil, formatDate, formatRelative, todayIso } from "#/lib";

function greeting() {
	const hour = new Date().getHours();
	return hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
}

const inDays = (days: number) =>
	new Date(Date.now() + days * 86_400_000).toISOString().slice(0, 10);

export function DashboardPage() {
	const me = useMe();
	const overdue = useInfiniteQuery(projectsQuery({ overdue: true }, 6));
	const dueSoon = useInfiniteQuery(
		projectsQuery({ stage: "ACTIVE", dueFrom: todayIso(), dueTo: inDays(14) }, 6),
	);
	const clients = useInfiniteQuery(clientsQuery({}, 6));

	return (
		<>
			<PageHeader
				eyebrow={`${roleLabel[me.role]} workspace`}
				title={`${greeting()}, ${me.displayName.split(" ")[0]}.`}
				description={
					me.role === "SALES"
						? "Your clients, the projects of your team, and what needs you first."
						: "Every client and project, with what needs attention first."
				}
			/>

			<div className="mb-8 grid gap-3 sm:grid-cols-3">
				<Shortcut to="/clients" icon={Building2} title="Clients" text="Search, add and update" />
				<Shortcut
					to="/projects"
					icon={BriefcaseBusiness}
					title="Projects"
					text="Stages, comments, history"
				/>
				<Shortcut to="/teams" icon={UsersRound} title="Teams" text="Members and delivery" />
			</div>

			<div className="grid gap-6 lg:grid-cols-2">
				<Panel
					icon={AlarmClock}
					title="Overdue"
					subtitle="Open projects after their due date"
					link={
						<Link
							to="/projects"
							search={{ overdue: true }}
							className="text-sm text-muted hover:text-foreground"
						>
							All overdue →
						</Link>
					}
				>
					<ProjectList query={overdue} empty="Nothing overdue. Well done." accent="danger" />
				</Panel>
				<Panel
					icon={CalendarRange}
					title="Due in two weeks"
					subtitle="Active projects with a deadline coming up"
					link={
						<Link
							to="/projects"
							search={{ stage: "ACTIVE" }}
							className="text-sm text-muted hover:text-foreground"
						>
							All active →
						</Link>
					}
				>
					<ProjectList query={dueSoon} empty="No active project is due in the next 14 days." />
				</Panel>
			</div>

			<Card className="mt-6">
				<Card.Header className="flex-row items-center justify-between">
					<div>
						<Card.Title className="font-display text-xl">Latest clients</Card.Title>
						<Card.Description>Newest first</Card.Description>
					</div>
					<Link to="/clients" className="text-sm text-muted hover:text-foreground">
						All clients →
					</Link>
				</Card.Header>
				<Card.Content>
					{clients.isPending ? (
						<Skeleton className="h-40 rounded-lg" />
					) : clients.isError ? (
						<ApiError error={clients.error} />
					) : (
						<ul className="grid gap-x-8 sm:grid-cols-2">
							{clients.data.pages[0]?.items.map((c) => (
								<li
									key={c.id}
									className="border-b border-separator last:border-0 sm:[&:nth-last-child(2)]:border-0"
								>
									<Link
										to="/clients/$clientId"
										params={{ clientId: c.id }}
										className="flex items-center gap-3 py-3 hover:opacity-80"
									>
										<span className="min-w-0 flex-1">
											<span className="block truncate text-sm font-medium">{c.name}</span>
											<span className="text-xs text-muted">
												{c.city ?? "—"} · {formatRelative(c.createdAt)}
											</span>
										</span>
										<ClientStatusChip status={c.status} />
									</Link>
								</li>
							))}
						</ul>
					)}
				</Card.Content>
			</Card>
		</>
	);
}

function Shortcut({
	to,
	icon: Icon,
	title,
	text,
}: {
	to: "/clients" | "/projects" | "/teams";
	icon: typeof Building2;
	title: string;
	text: string;
}) {
	return (
		<Link
			to={to}
			className="group flex items-center gap-4 rounded-xl border border-border bg-surface px-5 py-4 transition-colors hover:border-ink"
		>
			<span className="flex size-10 items-center justify-center rounded-lg bg-ink text-champagne">
				<Icon className="size-5" strokeWidth={1.75} />
			</span>
			<span className="flex-1">
				<span className="block font-display text-xl font-semibold">{title}</span>
				<span className="text-xs text-muted">{text}</span>
			</span>
			<ArrowRight className="size-4 text-muted transition-transform group-hover:translate-x-1" />
		</Link>
	);
}

function Panel({
	icon: Icon,
	title,
	subtitle,
	link,
	children,
}: {
	icon: typeof Building2;
	title: string;
	subtitle: string;
	link: ReactNode;
	children: ReactNode;
}) {
	return (
		<Card>
			<Card.Header className="flex-row items-start justify-between gap-4">
				<div className="flex items-start gap-3">
					<Icon className="mt-1 size-5 text-champagne" strokeWidth={1.75} />
					<div>
						<Card.Title className="font-display text-xl">{title}</Card.Title>
						<Card.Description>{subtitle}</Card.Description>
					</div>
				</div>
				{link}
			</Card.Header>
			<Card.Content>{children}</Card.Content>
		</Card>
	);
}

function ProjectList({
	query,
	empty,
	accent,
}: {
	query: UseInfiniteQueryResult<InfiniteData<PageResponseProjectResponse>, Error>;
	empty: string;
	accent?: "danger";
}) {
	if (query.isPending) return <Skeleton className="h-48 rounded-lg" />;
	if (query.isError) return <ApiError error={query.error} />;
	const items = query.data.pages[0]?.items ?? [];
	if (items.length === 0) return <p className="py-8 text-center text-sm text-muted">{empty}</p>;
	return (
		<ul className="flex flex-col">
			{items.map((p) => {
				const days = daysUntil(p.dueDate);
				return (
					<li key={p.id} className="border-b border-separator last:border-0">
						<Link
							to="/projects/$projectId"
							params={{ projectId: p.id }}
							className="flex items-center gap-3 py-3 hover:opacity-80"
						>
							<span className="min-w-0 flex-1">
								<span className="block truncate text-sm font-medium">{p.name}</span>
								<span className="block truncate text-xs text-muted">
									{p.clientName} · {p.teamName}
								</span>
							</span>
							<StageChip stage={p.stage} />
							<span
								className={`figures w-24 text-right text-xs ${accent === "danger" ? "font-semibold text-danger" : "text-foreground"}`}
								title={formatDate(p.dueDate)}
							>
								{days < 0 ? `${-days}d late` : days === 0 ? "today" : `in ${days}d`}
							</span>
						</Link>
					</li>
				);
			})}
		</ul>
	);
}
