import {
	type InfiniteData,
	infiniteQueryOptions,
	queryOptions,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";
import {
	countClients,
	createClient,
	deleteClient,
	getClient,
	searchClients,
	updateClient,
} from "#/api/endpoints/clients/clients";
import type {
	ClientRequest,
	ClientResponse,
	PageResponseClientSummary,
	SearchClientsParams,
} from "#/api/models";
import { clientKeys, projectKeys } from "#/api/query-keys";

export type ClientFilters = Omit<SearchClientsParams, "cursor" | "size">;

export const clientsQuery = (filters: ClientFilters, size = 25) =>
	infiniteQueryOptions<
		PageResponseClientSummary,
		Error,
		InfiniteData<PageResponseClientSummary>,
		ReturnType<typeof clientKeys.list>,
		string | undefined
	>({
		queryKey: clientKeys.list({ ...filters, size }),
		queryFn: ({ pageParam, signal }) =>
			searchClients({ ...filters, size, cursor: pageParam }, { signal }),
		initialPageParam: undefined,
		getNextPageParam: (page) => page.nextCursor ?? undefined,
	});

/** Total for the same filters as the list. Runs only on request (useOnDemandTotal): it reads every matching row. */
export const clientsCountQuery = (filters: ClientFilters) =>
	queryOptions({
		queryKey: clientKeys.count(filters),
		queryFn: ({ signal }) => countClients(filters, { signal }),
	});

export const clientQuery = (id: number) =>
	queryOptions({ queryKey: clientKeys.detail(id), queryFn: () => getClient(id) });

/** Every client write invalidates the whole area; projects show client names, so they refetch too. */
function useClientMutation<TVars, TResult>(
	fn: (vars: TVars) => Promise<TResult>,
	onSuccess?: (result: TResult) => void,
) {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: fn,
		onSuccess: async (result) => {
			await Promise.all([
				queryClient.invalidateQueries({ queryKey: clientKeys.all }),
				queryClient.invalidateQueries({ queryKey: projectKeys.all }),
			]);
			onSuccess?.(result);
		},
	});
}

export const useCreateClient = (onSuccess?: (c: ClientResponse) => void) =>
	useClientMutation((body: ClientRequest) => createClient(body), onSuccess);

export const useUpdateClient = (id: number, onSuccess?: (c: ClientResponse) => void) =>
	useClientMutation((body: ClientRequest) => updateClient(id, body), onSuccess);

export const useDeleteClient = (onSuccess?: () => void) =>
	useClientMutation(
		(c: { id: number; rowVersion: string }) => deleteClient(c.id, { rowVersion: c.rowVersion }),
		onSuccess,
	);
