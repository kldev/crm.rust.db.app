import type {
	CountClientsParams,
	CountProjectsParams,
	CountTicketsParams,
	ListDeskAgentsParams,
	ListUsersParams,
	SearchClientsParams,
	SearchProjectsParams,
	SearchTicketsParams,
} from "#/api/models";

/*
 * Query keys, one factory per area. A mutation invalidates the whole area (`xKeys.all`), so every list and
 * details page of it refetches; the lists are keyset pages, so there is nothing to patch by hand.
 */
export const authKeys = {
	all: ["auth"] as const,
	me: () => [...authKeys.all, "me"] as const,
	devLogin: () => [...authKeys.all, "dev-login"] as const,
};

export const clientKeys = {
	all: ["clients"] as const,
	list: (filters: Omit<SearchClientsParams, "cursor">) =>
		[...clientKeys.all, "list", filters] as const,
	count: (filters: CountClientsParams) => [...clientKeys.all, "count", filters] as const,
	detail: (id: number) => [...clientKeys.all, "detail", id] as const,
};

export const projectKeys = {
	all: ["projects"] as const,
	list: (filters: Omit<SearchProjectsParams, "cursor">) =>
		[...projectKeys.all, "list", filters] as const,
	count: (filters: CountProjectsParams) => [...projectKeys.all, "count", filters] as const,
	detail: (id: number) => [...projectKeys.all, "detail", id] as const,
	stages: (id: number) => [...projectKeys.all, "stages", id] as const,
	comments: (id: number) => [...projectKeys.all, "comments", id] as const,
};

export const teamKeys = {
	all: ["teams"] as const,
	list: () => [...teamKeys.all, "list"] as const,
	members: (id: number) => [...teamKeys.all, "members", id] as const,
	stats: (id: number) => [...teamKeys.all, "stats", id] as const,
};

export const userKeys = {
	all: ["users"] as const,
	list: (filters: Omit<ListUsersParams, "cursor">) => [...userKeys.all, "list", filters] as const,
};

/** Service desk. Every ticket action invalidates the whole area: lists, counters, the ticket and its timeline. */
export const deskKeys = {
	all: ["desk"] as const,
	catalog: () => [...deskKeys.all, "catalog"] as const,
	list: (filters: Omit<SearchTicketsParams, "cursor">) =>
		[...deskKeys.all, "list", filters] as const,
	count: (filters: CountTicketsParams) => [...deskKeys.all, "count", filters] as const,
	counters: () => [...deskKeys.all, "counters"] as const,
	detail: (id: number) => [...deskKeys.all, "detail", id] as const,
	events: (id: number) => [...deskKeys.all, "events", id] as const,
	attachments: (id: number) => [...deskKeys.all, "attachments", id] as const,
	fields: (id: number) => [...deskKeys.all, "fields", id] as const,
	linked: (id: number) => [...deskKeys.all, "linked", id] as const,
	report: (kind: string, filters: object) => [...deskKeys.all, "report", kind, filters] as const,
	agents: (filters: Omit<ListDeskAgentsParams, "cursor">) =>
		[...deskKeys.all, "agents", filters] as const,
	people: () => [...deskKeys.all, "people"] as const,
};

/** The public form (no session): its own keys, so signing in or out never mixes them with the agents' data. */
export const publicDeskKeys = {
	all: ["public-desk"] as const,
	catalog: () => [...publicDeskKeys.all, "catalog"] as const,
	people: () => [...publicDeskKeys.all, "people"] as const,
};
