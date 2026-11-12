import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { ProjectDetailsPage } from "#/features/projects/ProjectDetailsPage";

export const Route = createFileRoute("/_app/projects/$projectId")({
	params: {
		parse: (p) => ({ projectId: z.coerce.number().int().parse(p.projectId) }),
		stringify: (p) => ({ projectId: String(p.projectId) }),
	},
	validateSearch: z.object({ tab: z.enum(["comments", "history"]).optional() }),
	component: ProjectDetailsPage,
});
