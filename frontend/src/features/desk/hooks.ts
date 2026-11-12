import {
	type InfiniteData,
	infiniteQueryOptions,
	queryOptions,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";
import {
	assignTicketHandler,
	attachTicketFiles,
	changeTicketPriority,
	commentTicket,
	countTickets,
	getDeskCatalog,
	getTicket,
	getTicketCounters,
	handOverTicket,
	listDeskAgents,
	listDeskPeople,
	listLinkedTickets,
	listTicketAttachments,
	listTicketEvents,
	listTicketFields,
	reportTicket,
	resolveTicket,
	returnTicket,
	searchTickets,
	takeTicket,
	updateTicketFields,
} from "#/api/endpoints/desk/desk";
import { getPublicDeskCatalog, listPublicPeople } from "#/api/endpoints/public-desk/public-desk";
import type {
	AssignHandlerRequest,
	ChangePriorityRequest,
	HandOverRequest,
	PageResponseAgentSummary,
	PageResponseTicketResponse,
	PublicReportRequest,
	PublicReportResponse,
	ReportTicketRequest,
	SearchTicketsParams,
	TicketResponse,
} from "#/api/models";
import { customInstance } from "#/api/mutator";
import { deskKeys, publicDeskKeys } from "#/api/query-keys";
import type { TimelineEvent } from "./types";

export type TicketFilters = Omit<SearchTicketsParams, "cursor" | "size">;

/** The catalog changes only with a release (a migration), so it is fetched once per session. */
export const deskCatalogQuery = () =>
	queryOptions({
		queryKey: deskKeys.catalog(),
		queryFn: () => getDeskCatalog(),
		staleTime: Number.POSITIVE_INFINITY,
	});

export const publicCatalogQuery = () =>
	queryOptions({
		queryKey: publicDeskKeys.catalog(),
		queryFn: () => getPublicDeskCatalog(),
		staleTime: Number.POSITIVE_INFINITY,
	});

export const publicPeopleQuery = () =>
	queryOptions({ queryKey: publicDeskKeys.people(), queryFn: () => listPublicPeople() });

export const ticketsQuery = (filters: TicketFilters, size = 25) =>
	infiniteQueryOptions<
		PageResponseTicketResponse,
		Error,
		InfiniteData<PageResponseTicketResponse>,
		ReturnType<typeof deskKeys.list>,
		string | undefined
	>({
		queryKey: deskKeys.list({ ...filters, size }),
		queryFn: ({ pageParam, signal }) =>
			searchTickets({ ...filters, size, cursor: pageParam }, { signal }),
		initialPageParam: undefined,
		getNextPageParam: (page) => page.nextCursor ?? undefined,
		// Another agent may take a ticket meanwhile: lists refresh on their own, like the counters.
		refetchInterval: 30_000,
	});

/** Total for the same filters as the list. Runs only on request (useOnDemandTotal). */
export const ticketsCountQuery = (filters: TicketFilters) =>
	queryOptions({
		queryKey: deskKeys.count(filters),
		queryFn: ({ signal }) => countTickets(filters, { signal }),
	});

/** The counters of the top bar, refreshed every 30 s and after every action. */
export const countersQuery = () =>
	queryOptions({
		queryKey: deskKeys.counters(),
		queryFn: () => getTicketCounters(),
		refetchInterval: 30_000,
	});

export const ticketQuery = (id: number) =>
	queryOptions({ queryKey: deskKeys.detail(id), queryFn: () => getTicket(id) });

export const ticketEventsQuery = (id: number) =>
	queryOptions({
		queryKey: deskKeys.events(id),
		queryFn: async (): Promise<TimelineEvent[]> =>
			(await listTicketEvents(id)).map((e) => ({ ...e, data: JSON.parse(e.data) })),
	});

export const agentsQuery = (search: string | undefined, size = 50) =>
	infiniteQueryOptions<
		PageResponseAgentSummary,
		Error,
		InfiniteData<PageResponseAgentSummary>,
		ReturnType<typeof deskKeys.agents>,
		string | undefined
	>({
		queryKey: deskKeys.agents({ search, size }),
		queryFn: ({ pageParam, signal }) =>
			listDeskAgents({ search, size, cursor: pageParam }, { signal }),
		initialPageParam: undefined,
		getNextPageParam: (page) => page.nextCursor ?? undefined,
	});

export const attachmentsQuery = (id: number) =>
	queryOptions({ queryKey: deskKeys.attachments(id), queryFn: () => listTicketAttachments(id) });

/** Where an attachment opens: inline for images and PDF, `download` for a file to save. */
export const attachmentUrl = (ticketId: number, attachmentId: string, download = false) =>
	`/api/desk/tickets/${ticketId}/attachments/${attachmentId}${download ? "?download=true" : ""}`;

/**
 * The public report with its files in one multipart request. Written by hand: the generated client knows the form
 * fields but not the `files` part (springdoc does not merge @ModelAttribute with @RequestPart).
 */
function reportPublicWithFiles(body: PublicReportRequest, files: File[]) {
	const form = new FormData();
	for (const [key, value] of Object.entries(body)) {
		if (value !== undefined && value !== null) form.append(key, String(value));
	}
	for (const file of files) form.append("files", file);
	return customInstance<PublicReportResponse>({
		url: "/api/public/desk/tickets",
		method: "POST",
		data: form,
	});
}

export const ticketFieldsQuery = (id: number) =>
	queryOptions({ queryKey: deskKeys.fields(id), queryFn: () => listTicketFields(id) });

/** Marketing campaigns ordered for a ticket (their related-ticket is its number). */
export const linkedQuery = (id: number) =>
	queryOptions({ queryKey: deskKeys.linked(id), queryFn: () => listLinkedTickets(id) });

export const peopleQuery = () =>
	queryOptions({ queryKey: deskKeys.people(), queryFn: () => listDeskPeople() });

/** Every desk write invalidates the whole area, then calls onSuccess. */
function useDeskMutation<TVars, TResult>(
	fn: (vars: TVars) => Promise<TResult>,
	onSuccess?: (result: TResult) => void,
) {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: fn,
		onSuccess: async (result) => {
			await queryClient.invalidateQueries({ queryKey: deskKeys.all });
			onSuccess?.(result);
		},
	});
}

type OnTicket = (ticket: TicketResponse) => void;

export const useReportTicket = (onSuccess?: OnTicket) =>
	useDeskMutation((body: ReportTicketRequest) => reportTicket(body), onSuccess);

export const useReportPublicTicket = () =>
	useMutation({
		mutationFn: (v: { body: PublicReportRequest; files: File[] }) =>
			reportPublicWithFiles(v.body, v.files),
	});

export const useUpdateFields = (onSuccess?: OnTicket) =>
	useDeskMutation(
		(v: { id: number; values: Record<string, string> }) =>
			updateTicketFields(v.id, { values: v.values }),
		onSuccess,
	);

export const useAttachFiles = (onSuccess?: OnTicket) =>
	useDeskMutation(
		(v: { id: number; files: File[] }) => attachTicketFiles(v.id, { files: v.files }),
		onSuccess,
	);

export const useTakeTicket = (onSuccess?: OnTicket) =>
	useDeskMutation((id: number) => takeTicket(id), onSuccess);

export const useResolveTicket = (onSuccess?: OnTicket) =>
	useDeskMutation((id: number) => resolveTicket(id), onSuccess);

export const useReturnTicket = (onSuccess?: OnTicket) =>
	useDeskMutation(
		(v: { id: number; reason: string | null }) => returnTicket(v.id, { reason: v.reason }),
		onSuccess,
	);

export const useHandOverTicket = (onSuccess?: OnTicket) =>
	useDeskMutation((v: { id: number } & HandOverRequest) => handOverTicket(v.id, v), onSuccess);

export const useChangePriority = (onSuccess?: OnTicket) =>
	useDeskMutation(
		(v: { id: number } & ChangePriorityRequest) => changeTicketPriority(v.id, v),
		onSuccess,
	);

export const useCommentTicket = (onSuccess?: OnTicket) =>
	useDeskMutation(
		(v: { id: number; text: string }) => commentTicket(v.id, { text: v.text }),
		onSuccess,
	);

export const useAssignHandler = (onSuccess?: OnTicket) =>
	useDeskMutation(
		(v: { id: number } & AssignHandlerRequest) => assignTicketHandler(v.id, v),
		onSuccess,
	);
