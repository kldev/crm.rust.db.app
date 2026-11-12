import { Card, Label, Switch } from "@heroui/react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { getRouteApi } from "@tanstack/react-router";
import { BriefcaseBusiness } from "lucide-react";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, EmptyState, LoadMore, SelectFilter, TableSkeleton } from "#/components/ui";
import { teamsQuery } from "#/features/teams/hooks";
import { STAGES, type Stage, stageLabel } from "#/features/types";
import { useOnDemandTotal } from "#/hooks/useOnDemandTotal";
import { projectsCountQuery, projectsQuery } from "./hooks";
import { ProjectsTable } from "./ProjectsTable";

const route = getRouteApi("/_app/projects/");

export function ProjectsPage() {
	const search = route.useSearch();
	const navigate = route.useNavigate();
	const teams = useQuery(teamsQuery());
	const filters = { stage: search.stage, teamId: search.teamId, overdue: search.overdue };
	const query = useInfiniteQuery(projectsQuery(filters));
	const total = useOnDemandTotal(projectsCountQuery(filters));
	const projects = query.data?.pages.flatMap((p) => p.items) ?? [];

	return (
		<>
			<PageHeader
				eyebrow="Delivery"
				title="Projects"
				description="Every project you can see, nearest due date first. New projects start from a client's page."
			/>
			<Card className="overflow-hidden p-0">
				<div className="flex flex-col gap-3 border-b border-separator p-4 sm:flex-row sm:items-center">
					<SelectFilter
						label="Stage"
						value={search.stage}
						options={STAGES.map((s) => ({ id: s, label: stageLabel[s] }))}
						onChange={(stage) =>
							void navigate({ search: (s) => ({ ...s, stage: stage as Stage | undefined }) })
						}
					/>
					<SelectFilter
						label="Team"
						className="w-full sm:w-56"
						value={search.teamId ? String(search.teamId) : undefined}
						options={(teams.data ?? []).map((t) => ({ id: String(t.id), label: t.name }))}
						onChange={(teamId) =>
							void navigate({
								search: (s) => ({ ...s, teamId: teamId ? Number(teamId) : undefined }),
							})
						}
					/>
					<Switch
						className="sm:ml-auto"
						isSelected={Boolean(search.overdue)}
						onChange={(overdue) =>
							void navigate({ search: (s) => ({ ...s, overdue: overdue || undefined }) })
						}
					>
						<Switch.Control>
							<Switch.Thumb />
						</Switch.Control>
						<Switch.Content>
							<Label>Overdue only</Label>
						</Switch.Content>
					</Switch>
				</div>
				{query.isPending ? (
					<TableSkeleton />
				) : query.isError ? (
					<div className="p-4">
						<ApiError error={query.error} title="Projects could not be loaded" />
					</div>
				) : projects.length === 0 ? (
					<EmptyState
						icon={BriefcaseBusiness}
						title="No projects here"
						description="Clear a filter, or open a client to start a project."
					/>
				) : (
					<>
						<ProjectsTable projects={projects} />
						<LoadMore
							noun="projects"
							shown={projects.length}
							hasMore={query.hasNextPage}
							isLoading={query.isFetchingNextPage}
							onLoadMore={() => void query.fetchNextPage()}
							total={total}
						/>
					</>
				)}
			</Card>
		</>
	);
}
