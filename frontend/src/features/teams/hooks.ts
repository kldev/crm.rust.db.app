import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	addTeamMember,
	createTeam,
	getTeamStats,
	listTeamMembers,
	listTeams,
	removeTeamMember,
} from "#/api/endpoints/teams/teams";
import type { TeamResponse } from "#/api/models";
import { teamKeys } from "#/api/query-keys";

/** All teams (a few hundred): walked page by page once and cached; pickers and the teams page share it. */
export const teamsQuery = () =>
	queryOptions({
		queryKey: teamKeys.list(),
		queryFn: async () => {
			const teams: TeamResponse[] = [];
			let cursor: string | undefined;
			do {
				const page = await listTeams({ size: 200, cursor });
				teams.push(...page.items);
				cursor = page.nextCursor ?? undefined;
			} while (cursor);
			return teams;
		},
		staleTime: 5 * 60_000,
	});

export const teamMembersQuery = (id: number) =>
	queryOptions({ queryKey: teamKeys.members(id), queryFn: () => listTeamMembers(id) });

export const teamStatsQuery = (id: number) =>
	queryOptions({ queryKey: teamKeys.stats(id), queryFn: () => getTeamStats(id) });

export const useTeamMembers = (id: number | undefined) =>
	useQuery({ ...teamMembersQuery(id ?? 0), enabled: Boolean(id) });

function useTeamMutation<TVars, TResult>(
	fn: (v: TVars) => Promise<TResult>,
	onSuccess?: (r: TResult) => void,
) {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: fn,
		onSuccess: async (result) => {
			await queryClient.invalidateQueries({ queryKey: teamKeys.all });
			onSuccess?.(result);
		},
	});
}

export const useCreateTeam = (onSuccess?: (t: TeamResponse) => void) =>
	useTeamMutation((v: { name: string; leadUserId: number }) => createTeam(v), onSuccess);

export const useAddTeamMember = (teamId: number, onSuccess?: () => void) =>
	useTeamMutation((userId: number) => addTeamMember(teamId, { userId }), onSuccess);

export const useRemoveTeamMember = (teamId: number, onSuccess?: () => void) =>
	useTeamMutation((userId: number) => removeTeamMember(teamId, userId), onSuccess);
