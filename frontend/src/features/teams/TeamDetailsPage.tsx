import { Button, Card, Label, ListBox, Select, Skeleton, toast } from "@heroui/react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { getRouteApi, Link } from "@tanstack/react-router";
import { ArrowLeft, BriefcaseBusiness, UserMinus, UserPlus } from "lucide-react";
import { useState } from "react";
import { problemMessage } from "#/api/mutator";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, EmptyState, LoadMore, TableSkeleton, UserAvatar } from "#/components/ui";
import { useMe } from "#/features/auth/session";
import { projectsCountQuery, projectsQuery } from "#/features/projects/hooks";
import { ProjectsTable } from "#/features/projects/ProjectsTable";
import { type RoleName, roleLabel } from "#/features/types";
import { useSalesUsers } from "#/features/users/hooks";
import { useOnDemandTotal } from "#/hooks/useOnDemandTotal";
import {
	teamMembersQuery,
	teamStatsQuery,
	teamsQuery,
	useAddTeamMember,
	useRemoveTeamMember,
} from "./hooks";
import { TeamStats } from "./TeamStats";

const route = getRouteApi("/_app/teams/$teamId");

export function TeamDetailsPage() {
	const { teamId } = route.useParams();
	const me = useMe();
	const teams = useQuery(teamsQuery());
	const team = teams.data?.find((t) => t.id === teamId);
	const members = useQuery(teamMembersQuery(teamId));
	const stats = useQuery(teamStatsQuery(teamId));
	const projects = useInfiniteQuery(projectsQuery({ teamId }, 10));
	const projectsTotal = useOnDemandTotal(projectsCountQuery({ teamId }));
	// The procedures allow ADMIN and the team lead to change membership.
	const canManage = me.role === "ADMIN" || team?.leadUserId === me.id;

	if (teams.isPending) return <Skeleton className="h-40 w-full rounded-lg" />;
	if (!team)
		return <ApiError error={new Error("This team does not exist.")} title="Team not found" />;
	const list = projects.data?.pages.flatMap((p) => p.items) ?? [];

	return (
		<>
			<Link
				to="/teams"
				className="mb-6 inline-flex items-center gap-1.5 text-sm text-muted hover:text-foreground"
			>
				<ArrowLeft className="size-4" /> Teams
			</Link>
			<PageHeader
				eyebrow={<span className="figures">Team #{team.id}</span>}
				title={team.name}
				meta={
					<span className="flex items-center gap-2 text-sm text-muted">
						<UserAvatar name={team.leadLogin} /> led by{" "}
						<span className="font-mono text-xs text-foreground">{team.leadLogin}</span>
					</span>
				}
			/>

			<div className="grid gap-6 lg:grid-cols-2">
				<Card>
					<Card.Header>
						<Card.Title className="font-display text-xl">Delivery</Card.Title>
						<Card.Description>
							Read from an indexed view - a handful of rows, however many projects.
						</Card.Description>
					</Card.Header>
					<Card.Content>
						{stats.isPending ? (
							<Skeleton className="h-48 rounded-lg" />
						) : stats.isError ? (
							<ApiError error={stats.error} />
						) : (
							<TeamStats stats={stats.data} />
						)}
					</Card.Content>
				</Card>

				<Card>
					<Card.Header>
						<Card.Title className="font-display text-xl">Members</Card.Title>
						<Card.Description>
							{members.data ? `${members.data.length} people` : " "}
						</Card.Description>
					</Card.Header>
					<Card.Content className="flex flex-col gap-4">
						{canManage && members.data ? (
							<AddMember teamId={teamId} memberIds={members.data.map((m) => m.userId)} />
						) : null}
						{members.isPending ? (
							<Skeleton className="h-40 rounded-lg" />
						) : members.isError ? (
							<ApiError error={members.error} />
						) : (
							<ul className="flex flex-col divide-y divide-separator">
								{members.data.map((m) => (
									<MemberRow key={m.userId} teamId={teamId} canManage={canManage} member={m} />
								))}
							</ul>
						)}
					</Card.Content>
				</Card>
			</div>

			<Card className="mt-6 overflow-hidden p-0">
				<div className="border-b border-separator px-5 py-4">
					<h2 className="font-display text-xl font-semibold">Projects</h2>
				</div>
				{projects.isPending ? (
					<TableSkeleton rows={4} />
				) : list.length === 0 ? (
					<EmptyState icon={BriefcaseBusiness} title="No projects" />
				) : (
					<>
						<ProjectsTable projects={list} hide={["team"]} />
						<LoadMore
							noun="projects"
							shown={list.length}
							hasMore={projects.hasNextPage}
							isLoading={projects.isFetchingNextPage}
							onLoadMore={() => void projects.fetchNextPage()}
							total={projectsTotal}
						/>
					</>
				)}
			</Card>
		</>
	);
}

function MemberRow({
	teamId,
	canManage,
	member,
}: {
	teamId: number;
	canManage: boolean;
	member: { userId: number; login: string; displayName: string; role: string; active: boolean };
}) {
	const remove = useRemoveTeamMember(teamId, () =>
		toast.success(`${member.displayName} left the team`),
	);
	return (
		<li className="flex items-center gap-3 py-2.5">
			<UserAvatar name={member.displayName} />
			<span className="flex min-w-0 flex-1 flex-col leading-tight">
				<span className="truncate text-sm">{member.displayName}</span>
				<span className="font-mono text-[11px] text-muted">
					{member.login} · {roleLabel[member.role as RoleName] ?? member.role}
					{member.active ? "" : " · inactive"}
				</span>
			</span>
			{canManage ? (
				<Button
					isIconOnly
					size="sm"
					variant="ghost"
					aria-label={`Remove ${member.displayName}`}
					isPending={remove.isPending}
					onPress={() =>
						remove.mutate(member.userId, {
							// 51905: the member still owns open projects of the team.
							onError: (error) => toast.danger(problemMessage(error)),
						})
					}
				>
					<UserMinus className="size-4" />
				</Button>
			) : null}
		</li>
	);
}

function AddMember({ teamId, memberIds }: { teamId: number; memberIds: number[] }) {
	const sales = useSalesUsers(true);
	const [userId, setUserId] = useState<string | null>(null);
	const add = useAddTeamMember(teamId, () => {
		setUserId(null);
		toast.success("Member added");
	});
	const candidates = (sales.data ?? []).filter((u) => !memberIds.includes(u.id));
	return (
		<div className="flex flex-col gap-2">
			<div className="flex items-end gap-2">
				<Select
					className="flex-1"
					placeholder={sales.isPending ? "Loading…" : "Choose a sales user"}
					value={userId}
					onChange={(k) => setUserId(k == null ? null : String(k))}
				>
					<Label>Add a member</Label>
					<Select.Trigger>
						<Select.Value />
						<Select.Indicator />
					</Select.Trigger>
					<Select.Popover>
						<ListBox>
							{candidates.map((u) => (
								<ListBox.Item
									key={u.id}
									id={String(u.id)}
									textValue={`${u.displayName} ${u.login}`}
								>
									<span className="flex flex-col">
										<span>{u.displayName}</span>
										<span className="font-mono text-[11px] text-muted">{u.login}</span>
									</span>
									<ListBox.ItemIndicator />
								</ListBox.Item>
							))}
						</ListBox>
					</Select.Popover>
				</Select>
				<Button
					isDisabled={!userId}
					isPending={add.isPending}
					onPress={() => userId && add.mutate(Number(userId))}
				>
					<UserPlus className="size-4" /> Add
				</Button>
			</div>
			<ApiError error={add.error} />
		</div>
	);
}
