import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { ChartsPage } from "#/features/desk/charts/ChartsPage";

export const Route = createFileRoute("/_app/desk/charts")({
	staticData: { wide: true },
	validateSearch: z.object({
		range: z.enum(["DAYS_7", "DAYS_30", "WEEKS_12"]).optional(),
		category: z.string().optional(),
	}),
	component: ChartsPage,
});
