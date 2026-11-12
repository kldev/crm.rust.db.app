import {
	type InfiniteData,
	infiniteQueryOptions,
	useMutation,
	useQuery,
	useQueryClient,
} from "@tanstack/react-query";
import {
	changeUserRole,
	createUser,
	listUsers,
	resetUserPassword,
	setUserActive,
} from "#/api/endpoints/users/users";
import type { CreateUserRequest, PageResponseUserResponse, UserResponse } from "#/api/models";
import { userKeys } from "#/api/query-keys";

export const usersQuery = (role: string | undefined, size = 50) =>
	infiniteQueryOptions<
		PageResponseUserResponse,
		Error,
		InfiniteData<PageResponseUserResponse>,
		ReturnType<typeof userKeys.list>,
		string | undefined
	>({
		queryKey: userKeys.list({ role, size }),
		queryFn: ({ pageParam, signal }) => listUsers({ role, size, cursor: pageParam }, { signal }),
		initialPageParam: undefined,
		getNextPageParam: (page) => page.nextCursor ?? undefined,
	});

/** All users of a role (for pickers): the list is paged by id, so this walks the pages once. */
async function allUsers(role: string): Promise<UserResponse[]> {
	const users: UserResponse[] = [];
	let cursor: string | undefined;
	do {
		const page = await listUsers({ role, size: 200, cursor });
		users.push(...page.items);
		cursor = page.nextCursor ?? undefined;
	} while (cursor);
	return users.filter((u) => u.active);
}

export const useSalesUsers = (enabled: boolean) =>
	useQuery({
		queryKey: userKeys.list({ role: "SALES", size: -1 }),
		queryFn: () => allUsers("SALES"),
		enabled,
		staleTime: 5 * 60_000,
	});

export const useManagers = (enabled: boolean) =>
	useQuery({
		queryKey: userKeys.list({ role: "MANAGER", size: -1 }),
		queryFn: () => allUsers("MANAGER"),
		enabled,
		staleTime: 5 * 60_000,
	});

function useUserMutation<TVars, TResult>(
	fn: (v: TVars) => Promise<TResult>,
	onSuccess?: (r: TResult) => void,
) {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: fn,
		onSuccess: async (result) => {
			await queryClient.invalidateQueries({ queryKey: userKeys.all });
			onSuccess?.(result);
		},
	});
}

export const useCreateUser = (onSuccess?: (u: UserResponse) => void) =>
	useUserMutation((body: CreateUserRequest) => createUser(body), onSuccess);

export const useChangeUserRole = (onSuccess?: (u: UserResponse) => void) =>
	useUserMutation(
		(v: { id: number; role: string }) => changeUserRole(v.id, { role: v.role }),
		onSuccess,
	);

export const useSetUserActive = (onSuccess?: (u: UserResponse) => void) =>
	useUserMutation(
		(v: { id: number; active: boolean }) => setUserActive(v.id, { active: v.active }),
		onSuccess,
	);

export const useResetUserPassword = (onSuccess?: () => void) =>
	useUserMutation(
		(v: { id: number; password: string }) => resetUserPassword(v.id, { password: v.password }),
		onSuccess,
	);
