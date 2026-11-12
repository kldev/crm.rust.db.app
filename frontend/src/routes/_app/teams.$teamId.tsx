import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { TeamDetailsPage } from "#/features/teams/TeamDetailsPage";

export const Route = createFileRoute("/_app/teams/$teamId")({
	params: {
		parse: (p) => ({ teamId: z.coerce.number().int().parse(p.teamId) }),
		stringify: (p) => ({ teamId: String(p.teamId) }),
	},
	component: TeamDetailsPage,
});
