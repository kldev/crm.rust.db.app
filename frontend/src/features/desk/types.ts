import type { I18n } from "#/i18n";
import { TIME_ZONE } from "#/i18n";

/*
 * Words and rules of the service desk the UI needs (ADR-0007). The procedures are the authority; what is mirrored
 * here only formats and colours (the deadline state is a function of time, computed when shown, so a warning turns
 * into an alert without a reload).
 */

export const PRIORITIES = ["CRITICAL", "HIGH", "MEDIUM", "LOW"] as const;
export type Priority = (typeof PRIORITIES)[number];

export const STATUSES = ["IN_POOL", "IN_PROGRESS", "DONE"] as const;
export type Status = (typeof STATUSES)[number];

export const SCOPES = ["POOL", "MINE", "ALL"] as const;
export type Scope = (typeof SCOPES)[number];

export const DEADLINE_FILTERS = ["OVERDUE", "DUE_SOON"] as const;
export type DeadlineFilter = (typeof DEADLINE_FILTERS)[number];

export const COMPLETIONS = ["OPEN", "DONE"] as const;
export type Completion = (typeof COMPLETIONS)[number];

export const priorityLabel = (p: string, { t }: Pick<I18n, "t">) =>
	({
		CRITICAL: t("Krytyczny", "Critical"),
		HIGH: t("Wysoki", "High"),
		MEDIUM: t("Średni", "Medium"),
		LOW: t("Niski", "Low"),
	})[p as Priority] ?? p;

/** SLA as words; the minutes come from the catalog (crm.desk_priorities). */
export const slaLabel = (p: string, { t }: Pick<I18n, "t">) =>
	({
		CRITICAL: "3 h",
		HIGH: "8 h",
		MEDIUM: t("3 dni", "3 days"),
		LOW: t("1 tydzień", "1 week"),
	})[p as Priority] ?? "";

export const priorityColor: Record<Priority, "danger" | "warning" | "accent" | "default"> = {
	CRITICAL: "danger",
	HIGH: "warning",
	MEDIUM: "accent",
	LOW: "default",
};

export const statusLabel = (s: string, { t }: Pick<I18n, "t">) =>
	({
		IN_POOL: t("W puli", "In the pool"),
		IN_PROGRESS: t("W realizacji", "In progress"),
		DONE: t("Zrealizowane", "Resolved"),
	})[s as Status] ?? s;

export type DeadlineState = "NONE" | "ON_TIME" | "DUE_SOON" | "OVERDUE";

/** Under this much time left an open ticket is due soon (a warning). */
export const DUE_SOON_MINUTES = 60;

/** The deadline instant itself is already overdue; exactly 60 minutes left is still on time. */
export function deadlineState(status: string, deadline: string, now: number): DeadlineState {
	if (status === "DONE") return "NONE";
	const left = Date.parse(deadline) - now;
	if (left <= 0) return "OVERDUE";
	return left < DUE_SOON_MINUTES * 60_000 ? "DUE_SOON" : "ON_TIME";
}

/** A positive duration in days / hours / minutes, rounded down to the minute: "2 dni 5 h", "5 h 20 min", "45 min". */
export function durationText(ms: number, { lang }: Pick<I18n, "lang">): string {
	const minutes = Math.max(0, Math.floor(ms / 60_000));
	const perDay = 24 * 60;
	if (minutes >= perDay) {
		const days = Math.floor(minutes / perDay);
		const hours = Math.floor((minutes % perDay) / 60);
		const word = lang === "pl" ? (days === 1 ? "dzień" : "dni") : days === 1 ? "day" : "days";
		return `${days} ${word} ${hours} h`;
	}
	if (minutes >= 60) return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
	return `${minutes} min`;
}

/** Time left, or how late: "2 dni 5 h" / "po terminie o 2 h 10 min" (ADR-0007). */
export function timeLeftText(
	deadline: string,
	now: number,
	i18n: Pick<I18n, "lang" | "t">,
): string {
	const left = Date.parse(deadline) - now;
	if (left > 0) return durationText(left, i18n);
	if (-left < 60_000) return i18n.t("po terminie", "overdue");
	return i18n.t(
		`po terminie o ${durationText(-left, i18n)}`,
		`overdue by ${durationText(-left, i18n)}`,
	);
}

export function formatInstant(iso: string, { locale }: Pick<I18n, "locale">): string {
	return new Intl.DateTimeFormat(locale, {
		timeZone: TIME_ZONE,
		day: "2-digit",
		month: "short",
		year: "numeric",
		hour: "2-digit",
		minute: "2-digit",
	}).format(new Date(iso));
}

/** One event of a ticket's stream, its JSON parsed. */
export interface TimelineEvent {
	version: number;
	type: string;
	at: string;
	actorName?: string | null;
	data: Record<string, unknown>;
}

type Ref = { id?: number; name?: string } | null | undefined;

/** What happened, in words, and the free text that came with it (a reason, a comment). */
export function describeEvent(
	e: TimelineEvent,
	i18n: Pick<I18n, "t" | "n">,
): { title: string; who: string; text?: string } {
	const { t } = i18n;
	const d = e.data;
	const who = e.actorName ?? "";
	const reason = (d.reason as string | null | undefined) ?? undefined;
	switch (e.type) {
		case "TicketReported": {
			const requester = d.requester as { incognito?: boolean; name?: string | null } | undefined;
			const name = requester?.incognito ? "incognito" : (requester?.name ?? "");
			const priority = priorityLabel(String(d.priority), i18n);
			return d.channel === "SELF_SERVICE"
				? {
						title: t(
							`Zgłoszenie wysłane samodzielnie, priorytet ${priority}`,
							`Reported by the requester, priority ${priority}`,
						),
						who: name,
					}
				: {
						title: t(
							`Zgłoszenie zarejestrowane przez agenta w imieniu: ${name}, priorytet ${priority}`,
							`Registered by an agent on behalf of: ${name}, priority ${priority}`,
						),
						who,
					};
		}
		case "TicketTaken":
			return { title: t("Przejęcie z puli", "Taken from the pool"), who };
		case "TicketHandedOver": {
			const to = (d.to as Ref)?.name ?? "";
			return { title: t(`Przekazanie do: ${to}`, `Handed over to: ${to}`), who, text: reason };
		}
		case "TicketReturnedToPool":
			return { title: t("Oddanie do puli", "Returned to the pool"), who, text: reason };
		case "TicketPriorityChanged": {
			const from = priorityLabel(String(d.from), i18n);
			const to = priorityLabel(String(d.to), i18n);
			return {
				title: t(`Zmiana priorytetu: ${from} → ${to}`, `Priority changed: ${from} → ${to}`),
				who,
				text: reason,
			};
		}
		case "TicketResolved":
			return {
				title: d.late
					? t("Zrealizowane po terminie", "Resolved late")
					: t("Zrealizowane w terminie", "Resolved on time"),
				who,
			};
		case "TicketCommented":
			return { title: t("Komentarz", "Comment"), who, text: String(d.text ?? "") };
		case "TicketAttachmentAdded": {
			const name = String(d.fileName ?? "");
			return {
				title: t(`Załącznik: ${name}`, `Attachment: ${name}`),
				who: String(d.addedBy ?? who),
			};
		}
		case "TicketFieldsUpdated": {
			const changes =
				(d.changes as { label: string; from: string | null; to: string | null }[]) ?? [];
			const shown = (v: string | null) => (v ? i18n.n(v) : "-");
			return {
				title: t("Dane do raportu", "Report data"),
				who,
				text: changes
					.map((c) => `${i18n.n(c.label)}: ${shown(c.from)} → ${shown(c.to)}`)
					.join("; "),
			};
		}
		case "TicketHandlerAssigned": {
			const person = (d.person as Ref)?.name;
			return {
				title: person
					? t(`Do realizacji: ${person}`, `Person doing the work: ${person}`)
					: t("Bez osoby realizującej", "No person doing the work"),
				who,
				text: reason,
			};
		}
		default:
			return { title: e.type, who };
	}
}

/*
 * Attachment limits, mirrored from crm.desk_attachment_normalize and crm.desk_ticket_add_attachments (ADR-0009):
 * informative only - the picker refuses early, the procedure decides.
 */
export const ATTACHMENT_MAX_BYTES = 20 * 1024 * 1024;
export const ATTACHMENTS_PER_TICKET = 10;

/** "12,4 MB" / "830 KB". */
export function sizeText(bytes: number, { locale }: Pick<I18n, "locale">): string {
	if (bytes >= 1024 * 1024)
		return `${(bytes / 1024 / 1024).toLocaleString(locale, { maximumFractionDigits: 1 })} MB`;
	return `${Math.max(1, Math.floor(bytes / 1024))} KB`;
}

/** What to attach, said for the chosen team (as in Halcyon). */
export function attachmentHint(categoryKey: string | undefined, { t }: Pick<I18n, "t">): string {
	switch (categoryKey) {
		case "it":
			return t(
				"Np. zrzut ekranu z błędem aplikacji albo plik z logiem",
				"E.g. a screenshot of the application error or a log file",
			);
		case "marketing":
			return t(
				"Np. brief, logo, zdjęcia, materiały do kampanii",
				"E.g. a brief, logo, photos, campaign materials",
			);
		case "legal":
			return t(
				"Np. projekt umowy, korespondencja, pismo",
				"E.g. a draft contract, correspondence, a letter",
			);
		case "hr-abroad":
		case "hr-poland":
			return t(
				"Np. skan dokumentu, zaświadczenie, formularz",
				"E.g. a scanned document, a certificate, a form",
			);
		case "recruitment":
			return t(
				"Np. opis stanowiska, zamówienie klienta, CV kandydata",
				"E.g. a job description, the client's order, a candidate's CV",
			);
		case "finance":
			return t(
				"Np. skan faktury, rachunek, potwierdzenie przelewu",
				"E.g. a scanned invoice, a bill, a transfer confirmation",
			);
		case "legalization":
			return t(
				"Np. skan paszportu, wizy, umowy najmu, decyzji urzędu",
				"E.g. a scan of the passport, visa, tenancy agreement, office decision",
			);
		default:
			return t("Zrzuty ekranu, dokumenty, materiały", "Screenshots, documents, materials");
	}
}
