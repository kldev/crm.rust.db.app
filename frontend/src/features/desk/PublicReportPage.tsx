import { Button, Skeleton } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { CheckCircle2, Clock, EyeOff, Hash, LogIn, Plus } from "lucide-react";
import { type ReactNode, useState } from "react";
import { getCsrfToken } from "#/api/endpoints/auth/auth";
import type { PublicReportResponse } from "#/api/models";
import { ApiError } from "#/components/ui";
import { useI18n } from "#/i18n";
import { LanguageSwitch } from "#/i18n/LanguageSwitch";
import { ReportForm } from "./components/ReportForm";
import { publicCatalogQuery, publicPeopleQuery, useReportPublicTicket } from "./hooks";
import { formatInstant } from "./types";

/**
 * The public report form (no sign-in, ADR-0007): an employee reports a request to a team and gets its number.
 * The page shows names to pick from and creates tickets - nothing else. The form is the same wizard the agents use.
 */
export function PublicReportPage() {
	const i18n = useI18n();
	const { t } = i18n;
	const catalog = useQuery(publicCatalogQuery());
	const people = useQuery(publicPeopleQuery());
	const report = useReportPublicTicket();
	const [sent, setSent] = useState<PublicReportResponse | null>(null);
	// A new form after "report another": remounting resets every field.
	const [round, setRound] = useState(0);

	return (
		<div className="min-h-screen bg-background">
			<header className="sticky top-0 z-20 flex h-14 items-center gap-3 border-b border-border bg-topbar/95 px-4 backdrop-blur sm:px-8">
				<img src="/favicon.svg" alt="" className="size-7" />
				<span className="font-display text-base font-bold">R CRM</span>
				<span className="h-4 w-px bg-border" aria-hidden />
				<span className="font-mono text-[11px] tracking-[0.12em] text-muted uppercase">
					Service desk
				</span>
				<div className="ml-auto flex items-center gap-3">
					<LanguageSwitch />
					<Link
						to="/login"
						className="inline-flex items-center gap-1.5 rounded-md border border-border bg-surface px-3 py-1.5 text-sm text-foreground transition-colors hover:border-accent hover:text-accent"
					>
						<LogIn className="size-4" />
						<span className="hidden sm:inline">{t("Logowanie agentów", "Agents' sign-in")}</span>
					</Link>
				</div>
			</header>

			<section className="relative overflow-hidden bg-sidebar pt-10 pb-28 text-sidebar-text">
				<div
					aria-hidden
					className="pointer-events-none absolute inset-0 bg-[linear-gradient(to_right,rgb(56_198_224/0.07)_1px,transparent_1px),linear-gradient(to_bottom,rgb(56_198_224/0.07)_1px,transparent_1px)] bg-size-[40px_40px] [mask-image:radial-gradient(ellipse_at_top_right,black,transparent_75%)]"
				/>
				<div className="relative mx-auto w-full max-w-5xl px-4 sm:px-8">
					<p className="font-mono text-[11px] tracking-[0.16em] text-signal uppercase">
						{t("Bez logowania", "No sign-in needed")}
					</p>
					<h1 className="mt-3 font-display text-3xl font-bold text-white sm:text-4xl">
						{t("Zgłoś sprawę", "Report a request")}
					</h1>
					<p className="mt-2 max-w-2xl text-sm text-sidebar-text">
						{t(
							"Wybierz dział i temat, opisz sprawę i wyślij. Zgłoszenie trafia do puli zespołu, który je obsłuży.",
							"Pick the team and topic, describe the request and send it. It goes to the pool of the team that handles it.",
						)}
					</p>
					<ul className="mt-6 flex flex-wrap gap-x-8 gap-y-3 text-sm">
						<Fact icon={<Hash className="size-4" />}>
							{t("Od razu dostajesz numer", "You get a number at once")}
						</Fact>
						<Fact icon={<Clock className="size-4" />}>
							{t("Termin wynika z priorytetu", "The deadline follows the priority")}
						</Fact>
						<Fact icon={<EyeOff className="size-4" />}>
							{t("Możesz zgłosić incognito", "You may report incognito")}
						</Fact>
					</ul>
				</div>
			</section>

			<main className="relative mx-auto -mt-20 w-full max-w-5xl px-4 pb-16 sm:px-8">
				<div className="overflow-hidden rounded-lg border border-border bg-surface shadow-[0_8px_30px_rgba(15,23,42,0.08)]">
					{sent ? (
						<div className="flex flex-col items-center gap-4 px-6 py-14 text-center">
							<span className="flex size-14 items-center justify-center rounded-full bg-success-soft text-success">
								<CheckCircle2 className="size-7" />
							</span>
							<h2 className="font-display text-2xl font-bold">
								{t("Zgłoszenie wysłane", "Request sent")}
							</h2>
							<div className="flex flex-col items-center gap-1">
								<p className="font-mono text-[11px] tracking-[0.12em] text-muted uppercase">
									{t("Numer zgłoszenia", "Ticket number")}
								</p>
								<p
									className="rounded-md border border-border bg-surface-secondary px-4 py-2 font-mono text-2xl font-semibold"
									data-testid="ticket-number"
								>
									{sent.number}
								</p>
							</div>
							<p className="text-sm text-muted">
								{t("Termin realizacji", "Deadline")}:{" "}
								<strong className="figures text-foreground">
									{formatInstant(sent.deadline, i18n)}
								</strong>
							</p>
							<Button
								variant="secondary"
								onPress={() => {
									setSent(null);
									report.reset();
									setRound((r) => r + 1);
								}}
							>
								<Plus className="size-4" />
								{t("Zgłoś kolejną sprawę", "Report another request")}
							</Button>
						</div>
					) : catalog.isPending || people.isPending ? (
						<div className="p-8">
							<Skeleton className="h-96 w-full rounded-md" />
						</div>
					) : catalog.isError || people.isError ? (
						<div className="p-8">
							<ApiError error={catalog.error ?? people.error} />
						</div>
					) : (
						<ReportForm
							key={round}
							frame="page"
							mode="public"
							catalog={catalog.data}
							people={people.data}
							isPending={report.isPending}
							error={report.error}
							onSubmit={async (v, files, fields) => {
								// The first write needs the XSRF-TOKEN cookie; this request sets it.
								await getCsrfToken();
								report.mutate(
									{
										body: {
											title: v.title.trim(),
											description: v.description.trim(),
											categoryKey: v.categoryKey,
											subcategoryKey: v.subcategoryKey,
											priority: v.priority,
											requesterPersonId: v.requesterPersonId,
											requesterIncognito: v.requesterIncognito,
											fields: JSON.stringify(fields),
										},
										files,
									},
									{ onSuccess: setSent },
								);
							}}
						/>
					)}
				</div>
			</main>
		</div>
	);
}

function Fact({ icon, children }: { icon: ReactNode; children: ReactNode }) {
	return (
		<li className="flex items-center gap-2">
			<span className="text-signal">{icon}</span>
			{children}
		</li>
	);
}
