import { type QueryKey, type UseQueryOptions, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import type { TotalResponse } from "#/api/models";

export interface OnDemandTotal {
	/** Undefined until the user asks for it (or after a filter change). */
	total: number | undefined;
	isLoading: boolean;
	isError: boolean;
	request: () => void;
}

/*
 * A total the user asks for. Counting reads every matching row (millions with no filter), so the database must not
 * do it on every list load: the query runs only after `request()`, and only for the filters it was requested for.
 * Changing a filter hides the total until the user asks again. The count queries are never stale on their own
 * (staleTime: Infinity), so a total counted earlier for the same filters is shown from the cache without a new
 * request; after a write invalidates the area, the cached number is no longer trusted and is hidden.
 */
export function useOnDemandTotal<TKey extends QueryKey>(
	options: UseQueryOptions<TotalResponse, Error, TotalResponse, TKey>,
): OnDemandTotal {
	const key = JSON.stringify(options.queryKey);
	const [requestedFor, setRequestedFor] = useState<string | null>(null);
	const requested = requestedFor === key;
	const query = useQuery({ ...options, staleTime: Number.POSITIVE_INFINITY, enabled: requested });
	const trusted = requested || !query.isStale;

	return {
		total: trusted ? query.data?.total : undefined,
		isLoading: query.isFetching,
		isError: requested && query.isError,
		request: () => setRequestedFor(key),
	};
}
