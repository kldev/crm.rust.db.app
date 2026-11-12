import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";
import { NewTicketPage } from "#/features/desk/NewTicketPage";

// A link may fill the form: "order a campaign" from a recruitment ticket sets the topic, text and the related ticket.
export const Route = createFileRoute("/_app/desk/new")({
	validateSearch: z.object({
		sub: z.string().optional(),
		title: z.string().optional(),
		desc: z.string().optional(),
		related: z.string().optional(),
	}),
	component: NewTicketPage,
});
