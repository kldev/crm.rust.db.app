import { z } from "zod";
import { COMPLETIONS, DEADLINE_FILTERS, PRIORITIES } from "./types";

/** Filters of the tickets lists, kept in the URL (a link opens the same list). */
export const ticketSearchSchema = z.object({
	q: z.string().optional(),
	category: z.string().optional(),
	sub: z.string().optional(),
	priority: z.enum(PRIORITIES).optional(),
	deadline: z.enum(DEADLINE_FILTERS).optional(),
	completion: z.enum(COMPLETIONS).optional(),
});
export type TicketSearch = z.infer<typeof ticketSearchSchema>;
