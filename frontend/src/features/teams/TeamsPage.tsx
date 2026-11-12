import { Button, Card, SearchField, Skeleton, toast } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { getRouteApi, Link } from "@tanstack/react-router";
import { Plus, UsersRound } from "lucide-react";
import { useMemo, useRef } from "react";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, EmptyState, UserAvatar } from "#/components/ui";
import { can, useMe } from "#/features/auth/session";
import { type CreateTeamCommand, CreateTeamDrawer } from "./CreateTeamDrawer";
import { teamsQuery } from "./hooks";

const route = getRouteApi("/_app/teams/");
const PAGE = 60;

export function TeamsPage() {
	const me = useMe();
	const search = route.useSearch();
	const navigate = route.useNavigate();
	const teams = useQuery(teamsQuery());
	const drawer = useRef<CreateTeamCommand>(null);

	const filtered = useMemo(() => {
		const q = (search.q ?? "").trim().toLowerCase();
		const all = teams.data ?? [];
		return q ? all.filter((t) => t.name.toLowerCase().includes(q) || t.leadLogin.includes(q)) : all;
	}, [teams.data, search.q]);

	return (
		<>
			<PageHeader
				eyebrow="People"
				title="Teams"
				description="Each team is led by a manager and delivers projects; a project is owned by one of its members."
				actions={
					can.createTeams(me.role) ? (
						<Button onPress={() => drawer.current?.open()}>
							<Plus className="size-4" /> New team
						</Button>
					) : null
				}
			/>
			<div className="mb-5 flex items-center gap-3">
				<SearchField
					aria-label="Filter teams"
					className="w-full sm:max-w-xs"
					value={search.q ?? ""}
					onChange={(q) => void navigate({ search: { q: q || undefined }, replace: true })}
				>
					<SearchField.Group>
						<SearchField.SearchIcon />
						<SearchField.Input placeholder="Team or lead…" />
						<SearchField.ClearButton />
					</SearchField.Group>
				</SearchField>
				{teams.data ? (
					<span className="font-mono text-xs text-muted">
						{filtered.length.toLocaleString("en")} of {teams.data.length.toLocaleString("en")} teams
					</span>
				) : null}
			</div>

			{teams.isPending ? (
				<div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
					{Array.from({ length: 6 }, (_, i) => (
						<Skeleton key={i} className="h-36 rounded-lg" />
					))}
				</div>
			) : teams.isError ? (
				<ApiError error={teams.error} title="Teams could not be loaded" />
			) : filtered.length === 0 ? (
				<Card>
					<EmptyState icon={UsersRound} title="No teams found" description="Try another name." />
				</Card>
			) : (
				<>
					<div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
						{filtered.slice(0, PAGE).map((team) => (
							<Link
								key={team.id}
								to="/teams/$teamId"
								params={{ teamId: team.id }}
								className="group"
							>
								<Card className="h-full transition-shadow group-hover:shadow-md">
									<Card.Header>
										<Card.Title className="font-display text-2xl">{team.name}</Card.Title>
										<Card.Description className="font-mono text-[11px] tracking-[0.08em] uppercase">
											Team #{team.id}
										</Card.Description>
									</Card.Header>
									<Card.Content className="flex items-center justify-between">
										<span className="flex items-center gap-2">
											<UserAvatar name={team.leadLogin} />
											<span className="flex flex-col leading-tight">
												<span className="text-[11px] text-muted">Lead</span>
												<span className="font-mono text-xs">{team.leadLogin}</span>
											</span>
										</span>
										<span className="text-right">
											<span className="block font-display text-3xl leading-none font-semibold">
												{team.memberCount}
											</span>
											<span className="text-[11px] text-muted">members</span>
										</span>
									</Card.Content>
								</Card>
							</Link>
						))}
					</div>
					{filtered.length > PAGE ? (
						<p className="mt-4 text-center text-xs text-muted">
							Showing {PAGE} of {filtered.length} - narrow the filter to find a team.
						</p>
					) : null}
				</>
			)}
			<CreateTeamDrawer ref={drawer} onCreated={(t) => toast.success(`Team "${t.name}" created`)} />
		</>
	);
}
