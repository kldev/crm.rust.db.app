import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { TeamsPage } from "#/features/teams/TeamsPage";

export const Route = createFileRoute("/_app/teams/")({
	validateSearch: z.object({ q: z.string().optional() }),
	component: TeamsPage,
});
