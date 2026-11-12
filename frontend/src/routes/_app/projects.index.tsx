import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { ProjectsPage } from "#/features/projects/ProjectsPage";
import { STAGES } from "#/features/types";

export const Route = createFileRoute("/_app/projects/")({
	validateSearch: z.object({
		stage: z.enum(STAGES).optional(),
		teamId: z.coerce.number().int().optional(),
		overdue: z.boolean().optional(),
	}),
	component: ProjectsPage,
});
