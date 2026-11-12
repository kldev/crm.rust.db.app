import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { ClientDetailsPage } from "#/features/clients/ClientDetailsPage";

export const Route = createFileRoute("/_app/clients/$clientId")({
	params: {
		parse: (p) => ({ clientId: z.coerce.number().int().parse(p.clientId) }),
		stringify: (p) => ({ clientId: String(p.clientId) }),
	},
	component: ClientDetailsPage,
});
