import { queryOptions, useQueryClient, useSuspenseQuery } from "@tanstack/react-query";
import { getMe } from "#/api/endpoints/auth/auth";
import type { MeResponse } from "#/api/models";
import { authKeys } from "#/api/query-keys";

export type Role = "ADMIN" | "MANAGER" | "SALES";

/** The logged-in user; `null` when there is no session (the API answers 401). */
export const meQuery = () =>
	queryOptions({
		queryKey: authKeys.me(),
		queryFn: async (): Promise<MeResponse | null> => {
			try {
				return await getMe();
			} catch {
				return null;
			}
		},
		staleTime: 5 * 60_000,
	});

/** Inside the authenticated layout the user is always there (the route guard loaded it). */
export function useMe(): MeResponse & { role: Role } {
	const { data } = useSuspenseQuery(meQuery());
	if (!data) throw new Error("useMe outside the authenticated layout");
	return data as MeResponse & { role: Role };
}

export function useForgetSession() {
	const queryClient = useQueryClient();
	return () => {
		queryClient.setQueryData(authKeys.me(), null);
		queryClient.removeQueries({ predicate: (q) => q.queryKey[0] !== authKeys.all[0] });
	};
}

/** What a role may do in the UI. The procedures enforce the same rules; this only hides dead ends. */
export const can = {
	manageUsers: (role: Role) => role === "ADMIN",
	seeUsers: (role: Role) => role === "ADMIN" || role === "MANAGER",
	deleteClients: (role: Role) => role === "ADMIN" || role === "MANAGER",
	assignClients: (role: Role) => role !== "SALES",
	createTeams: (role: Role) => role === "ADMIN",
};
