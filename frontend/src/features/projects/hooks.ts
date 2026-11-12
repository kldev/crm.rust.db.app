import {
	type InfiniteData,
	infiniteQueryOptions,
	queryOptions,
	useMutation,
	useQueryClient,
} from "@tanstack/react-query";
import {
	addProjectComment,
	changeProjectStage,
	countProjects,
	createProject,
	getProject,
	listProjectComments,
	listProjectStages,
	searchProjects,
	updateProject,
} from "#/api/endpoints/projects/projects";
import type {
	ChangeStageRequest,
	CreateProjectRequest,
	PageResponseCommentResponse,
	PageResponseProjectResponse,
	ProjectResponse,
	SearchProjectsParams,
	UpdateProjectRequest,
} from "#/api/models";
import { clientKeys, projectKeys, teamKeys } from "#/api/query-keys";

export type ProjectFilters = Omit<SearchProjectsParams, "cursor" | "size">;

export const projectsQuery = (filters: ProjectFilters, size = 25) =>
	infiniteQueryOptions<
		PageResponseProjectResponse,
		Error,
		InfiniteData<PageResponseProjectResponse>,
		ReturnType<typeof projectKeys.list>,
		string | undefined
	>({
		queryKey: projectKeys.list({ ...filters, size }),
		queryFn: ({ pageParam, signal }) =>
			searchProjects({ ...filters, size, cursor: pageParam }, { signal }),
		initialPageParam: undefined,
		getNextPageParam: (page) => page.nextCursor ?? undefined,
	});

/** Total for the same filters as the list. Runs only on request (useOnDemandTotal): it reads every matching row. */
export const projectsCountQuery = (filters: ProjectFilters) =>
	queryOptions({
		queryKey: projectKeys.count(filters),
		queryFn: ({ signal }) => countProjects(filters, { signal }),
	});

export const projectQuery = (id: number) =>
	queryOptions({ queryKey: projectKeys.detail(id), queryFn: () => getProject(id) });

export const projectStagesQuery = (id: number) =>
	queryOptions({ queryKey: projectKeys.stages(id), queryFn: () => listProjectStages(id) });

export const projectCommentsQuery = (id: number) =>
	infiniteQueryOptions<
		PageResponseCommentResponse,
		Error,
		InfiniteData<PageResponseCommentResponse>,
		ReturnType<typeof projectKeys.comments>,
		string | undefined
	>({
		queryKey: projectKeys.comments(id),
		queryFn: ({ pageParam, signal }) =>
			listProjectComments(id, { size: 20, cursor: pageParam }, { signal }),
		initialPageParam: undefined,
		getNextPageParam: (page) => page.nextCursor ?? undefined,
	});

/** Project writes change lists, details, history and comments, and team statistics (indexed view). */
function useProjectMutation<TVars, TResult>(
	fn: (v: TVars) => Promise<TResult>,
	onSuccess?: (r: TResult) => void,
) {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: fn,
		onSuccess: async (result) => {
			await Promise.all([
				queryClient.invalidateQueries({ queryKey: projectKeys.all }),
				queryClient.invalidateQueries({ queryKey: teamKeys.all }),
				queryClient.invalidateQueries({ queryKey: clientKeys.all }),
			]);
			onSuccess?.(result);
		},
	});
}

export const useCreateProject = (onSuccess?: (p: ProjectResponse) => void) =>
	useProjectMutation((body: CreateProjectRequest) => createProject(body), onSuccess);

export const useUpdateProject = (id: number, onSuccess?: (p: ProjectResponse) => void) =>
	useProjectMutation((body: UpdateProjectRequest) => updateProject(id, body), onSuccess);

export const useChangeProjectStage = (id: number, onSuccess?: (p: ProjectResponse) => void) =>
	useProjectMutation((body: ChangeStageRequest) => changeProjectStage(id, body), onSuccess);

export const useAddComment = (id: number, onSuccess?: () => void) =>
	useProjectMutation((body: string) => addProjectComment(id, { body }), onSuccess);
