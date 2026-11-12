import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { ClientsPage } from "#/features/clients/ClientsPage";
import { CLIENT_STATUSES } from "#/features/types";

export const Route = createFileRoute("/_app/clients/")({
	validateSearch: z.object({
		q: z.string().optional(),
		city: z.string().optional(),
		status: z.enum(CLIENT_STATUSES).optional(),
	}),
	component: ClientsPage,
});
