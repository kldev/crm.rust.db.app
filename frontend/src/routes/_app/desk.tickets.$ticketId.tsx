import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { TicketDetailsPage } from "#/features/desk/TicketDetailsPage";

export const Route = createFileRoute("/_app/desk/tickets/$ticketId")({
	params: {
		parse: (p) => ({ ticketId: z.coerce.number().int().parse(p.ticketId) }),
		stringify: (p) => ({ ticketId: String(p.ticketId) }),
	},
	component: TicketDetailsPage,
});
