import { z } from "zod";

const isoDate = z.string().regex(/^\d{4}-\d{2}-\d{2}$/);

/** Filters of a category report, in the URL. Without dates: this month so far (Warsaw days). */
export const categoryReportSearch = z.object({
	from: isoDate.optional(),
	to: isoDate.optional(),
	market: z.string().optional(),
	sub: z.string().optional(),
	completion: z.enum(["OPEN", "DONE"]).optional(),
});
export type CategoryReportSearch = z.infer<typeof categoryReportSearch>;

export const agentReportSearch = z.object({ previous: z.boolean().optional() });

/** Today in Warsaw as yyyy-mm-dd. */
export function warsawToday(): string {
	return new Intl.DateTimeFormat("sv-SE", { timeZone: "Europe/Warsaw" }).format(new Date());
}

export const REPORTS = [
	{ slug: "kadry-polska", categoryKey: "hr-poland" },
	{ slug: "kadry-zagranica", categoryKey: "hr-abroad" },
	{ slug: "finanse", categoryKey: "finance" },
	{ slug: "rekrutacja", categoryKey: "recruitment" },
	{ slug: "marketing", categoryKey: "marketing" },
] as const;
