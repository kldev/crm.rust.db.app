import {
	Button,
	Checkbox,
	Description,
	ListBox,
	Modal,
	Radio,
	RadioGroup,
	SearchField,
} from "@heroui/react";
import {
	Check,
	FileText,
	Gauge,
	type LucideIcon,
	Paperclip,
	Shapes,
	UserRound,
} from "lucide-react";
import { type ReactNode, useMemo, useState } from "react";
import { z } from "zod";
import type { DeskCatalog, PublicPerson } from "#/api/models";
import { ApiError, DateInput } from "#/components/ui";
import { useAppForm } from "#/forms";
import { type I18n, useI18n } from "#/i18n";
import {
	ATTACHMENTS_PER_TICKET,
	attachmentHint,
	formatInstant,
	PRIORITIES,
	priorityLabel,
	slaLabel,
} from "../types";
import { AttachmentPicker } from "./AttachmentPicker";
import { CategoryIcon } from "./CategoryIcon";
import { FieldInputs, type FieldValues } from "./FieldInputs";

export interface ReportValues {
	categoryKey: string;
	subcategoryKey: string;
	title: string;
	description: string;
	priority: string;
	requesterPersonId: number | null;
	requesterIncognito: boolean;
	/** "YYYY-MM-DDTHH:mm" in the browser's time zone; only on the agent's form. */
	reportedAt: string;
}

/** A "YYYY-MM-DDTHH:mm" value (the browser's time) for now, or offsetDays from now, to the minute. */
export function localNow(offsetDays = 0): string {
	const d = new Date(Date.now() + offsetDays * 86_400_000);
	d.setSeconds(0, 0);
	return new Date(d.getTime() - d.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
}

/**
 * The report form (ADR-0007), the same for the agent and for the public page: category tiles, then the subcategory
 * (grouped by market), title and description, priority with its SLA and the deadline before sending, and the
 * requester - a person from the list or incognito. The agent's form also takes the report date (up to 30 days back).
 * zod checks what the form can know; the procedure is the authority and its refusal shows under the form.
 *
 * It is a wizard: a step at a time, "Next" checks only the step's fields, so a missing title never blocks choosing the
 * topic. frame "dialog" fills a Modal.Dialog (the signed-in agent), frame "page" a card on the public page.
 */
export function ReportForm({
	catalog,
	people,
	mode,
	isPending,
	error,
	onSubmit,
	initial,
	frame,
	onCancel,
}: {
	catalog: DeskCatalog;
	people: PublicPerson[];
	mode: "agent" | "public";
	isPending: boolean;
	error: unknown;
	onSubmit: (values: ReportValues, files: File[], fields: FieldValues) => void;
	/** Values to start from (a link such as "order a campaign" fills them). */
	initial?: Partial<
		Pick<ReportValues, "categoryKey" | "subcategoryKey" | "title" | "description">
	> & {
		fields?: FieldValues;
	};
	/** dialog: body and footer of a Modal.Dialog; page: a card of its own. */
	frame: "dialog" | "page";
	/** "Cancel" on the first step (the dialog). */
	onCancel?: () => void;
}) {
	const i18n = useI18n();
	const { t, n } = i18n;
	const [files, setFiles] = useState<File[]>([]);
	const [fieldValues, setFieldValues] = useState<FieldValues>(initial?.fields ?? {});
	const definitionsOf = (subcategoryKey: string) =>
		catalog.categories.flatMap((c) => c.subcategories).find((s) => s.key === subcategoryKey)
			?.fields ?? [];
	const base = useMemo(
		() =>
			z.object({
				categoryKey: z.string().min(1, t("Wybierz kategorię", "Choose a category")),
				subcategoryKey: z.string().min(1, t("Wybierz podkategorię", "Choose a subcategory")),
				title: z
					.string()
					.trim()
					.min(1, t("Tytuł jest wymagany", "Title is required"))
					.max(200, t("Najwyżej 200 znaków", "At most 200 characters")),
				description: z
					.string()
					.trim()
					.min(1, t("Opis jest wymagany", "Description is required"))
					.max(4000, t("Najwyżej 4000 znaków", "At most 4000 characters")),
				priority: z.string().min(1, t("Wybierz priorytet", "Choose a priority")),
				requesterPersonId: z.number().nullable(),
				requesterIncognito: z.boolean(),
				reportedAt: z.string(),
			}),
		[t],
	);
	const schema = useMemo(
		() =>
			base
				.refine((v) => v.requesterIncognito || v.requesterPersonId != null, {
					path: ["requesterPersonId"],
					message: t(
						"Wybierz zgłaszającego albo zaznacz „incognito”",
						"Choose the requester or mark “incognito”",
					),
				})
				.refine((v) => mode === "public" || v.reportedAt !== "", {
					path: ["reportedAt"],
					message: t("Podaj datę zgłoszenia", "Enter the report date"),
				}),
		[base, t, mode],
	);
	const [step, setStep] = useState(0);
	const stages = wizardStages(i18n, mode);
	/** Fields of the given keys the form would refuse now (the refinements included). */
	const invalidOf = (keys: readonly (keyof ReportValues)[]) => {
		const v = form.state.values;
		const parsed = base.safeParse(v);
		const bad = new Set(parsed.success ? [] : parsed.error.issues.map((i) => String(i.path[0])));
		if (!v.requesterIncognito && v.requesterPersonId == null) bad.add("requesterPersonId");
		if (mode === "agent" && v.reportedAt === "") bad.add("reportedAt");
		return keys.filter((k) => bad.has(k));
	};
	const next = () => {
		const bad = invalidOf(stages[step].fields);
		if (bad.length === 0) {
			setStep(step + 1);
			return;
		}
		for (const k of bad) form.setFieldMeta(k, (m) => ({ ...m, isTouched: true }));
		void form.validate("change");
	};
	const at = (i: number) => step === i;

	const form = useAppForm({
		defaultValues: {
			categoryKey: initial?.categoryKey ?? "",
			subcategoryKey: initial?.subcategoryKey ?? "",
			title: initial?.title ?? "",
			description: initial?.description ?? "",
			priority: "MEDIUM",
			requesterPersonId: null,
			requesterIncognito: false,
			reportedAt: mode === "agent" ? localNow() : "",
		} as ReportValues,
		validators: { onChange: schema },
		// A refusal sends the user back to the first step with a wrong field.
		onSubmitInvalid: () => {
			const first = stages.findIndex((st) => invalidOf(st.fields).length > 0);
			if (first >= 0) setStep(first);
		},
		// Only the chosen subcategory's fields travel; values typed for another topic are dropped.
		onSubmit: ({ value }) => {
			const keys = new Set(definitionsOf(value.subcategoryKey).map((f) => f.key));
			onSubmit(
				value,
				files,
				Object.fromEntries(
					Object.entries(fieldValues).filter(([k, v]) => keys.has(k) && v.trim() !== ""),
				),
			);
		},
	});

	const sections = (
		<>
			{at(0) ? (
				<Step title={t("Czego dotyczy sprawa?", "What is it about?")}>
					<form.Field name="categoryKey">
						{(field) => (
							<div className="flex flex-col gap-2">
								<div className="grid grid-cols-2 gap-3 md:grid-cols-4">
									{catalog.categories.map((c) => {
										const chosen = field.state.value === c.key;
										const markets = [
											...new Set(c.subcategories.map((s) => s.market).filter(Boolean)),
										];
										return (
											<button
												key={c.key}
												type="button"
												aria-pressed={chosen}
												onClick={() => {
													field.handleChange(c.key);
													form.setFieldValue("subcategoryKey", "");
												}}
												className={`flex flex-col items-start gap-2.5 rounded-md border p-4 text-left transition-all ${
													chosen
														? "border-accent bg-surface ring-2 ring-accent/25"
														: "border-separator bg-surface hover:border-accent/40 hover:shadow-sm"
												}`}
											>
												<CategoryIcon categoryKey={c.key} />
												<span className="text-sm font-medium text-foreground">{n(c.name)}</span>
												<span className="line-clamp-2 text-xs text-muted">
													{markets.length > 0
														? markets.map((m) => n(m)).join(" · ")
														: c.subcategories
																.filter((s) => s.active)
																.slice(0, 3)
																.map((s) => n(s.name))
																.join(", ")}
												</span>
											</button>
										);
									})}
								</div>
								<FieldMessage errors={field.state.meta.isTouched ? field.state.meta.errors : []} />
							</div>
						)}
					</form.Field>
				</Step>
			) : null}

			{at(0) ? (
				<form.Subscribe selector={(s) => s.values.categoryKey}>
					{(categoryKey) => {
						const category = catalog.categories.find((c) => c.key === categoryKey);
						if (!category) return null;
						const groups = [
							...new Set(category.subcategories.filter((s) => s.active).map((s) => s.market ?? "")),
						];
						return (
							<Step title={t("Temat", "Topic")}>
								<form.Field name="subcategoryKey">
									{(field) => (
										<div className="flex flex-col gap-4">
											{groups.map((group) => (
												<div key={group || "-"} className="flex flex-col gap-2">
													{group ? (
														<p className="font-mono text-[11px] tracking-[0.12em] text-muted uppercase">
															{n(group)}
														</p>
													) : null}
													<div className="flex flex-wrap gap-2">
														{category.subcategories
															.filter((s) => s.active && (s.market ?? "") === group)
															.map((s) => (
																<Button
																	key={s.key}
																	size="sm"
																	variant={field.state.value === s.key ? "primary" : "secondary"}
																	onPress={() => field.handleChange(s.key)}
																>
																	{n(s.name)}
																</Button>
															))}
													</div>
												</div>
											))}
											<FieldMessage
												errors={field.state.meta.isTouched ? field.state.meta.errors : []}
											/>
										</div>
									)}
								</form.Field>
							</Step>
						);
					}}
				</form.Subscribe>
			) : null}

			{at(0) ? (
				<form.Subscribe selector={(st) => st.values.subcategoryKey}>
					{(subcategoryKey) => {
						const definitions = definitionsOf(subcategoryKey);
						if (definitions.length === 0) return null;
						return (
							<Step title={t("Dane do raportu", "Report data")}>
								<FieldInputs fields={definitions} values={fieldValues} onChange={setFieldValues} />
							</Step>
						);
					}}
				</form.Subscribe>
			) : null}

			{at(1) ? (
				<Step title={t("Opis sprawy", "The request")}>
					<div className="flex flex-col gap-4">
						<form.AppField name="title">
							{(field) => (
								<field.FormInput
									label={t("Tytuł", "Title")}
									placeholder={t("Krótko, czego potrzebujesz", "In short, what you need")}
								/>
							)}
						</form.AppField>
						<form.AppField name="description">
							{(field) => (
								<field.FormTextArea
									label={t("Opis", "Description")}
									rows={5}
									placeholder={t(
										"Co się stało, od kiedy, kogo dotyczy…",
										"What happened, since when, who it affects…",
									)}
								/>
							)}
						</form.AppField>
					</div>
				</Step>
			) : null}

			{at(2) ? (
				<Step title={t("Priorytet", "Priority")}>
					<form.Field name="priority">
						{(field) => (
							<RadioGroup
								aria-label={t("Priorytet", "Priority")}
								orientation="horizontal"
								value={field.state.value}
								onChange={field.handleChange}
							>
								{PRIORITIES.map((p) => (
									<Radio key={p} value={p}>
										<Radio.Content>
											<Radio.Control>
												<Radio.Indicator />
											</Radio.Control>
											{priorityLabel(p, i18n)}
										</Radio.Content>
										<Description>{slaLabel(p, i18n)}</Description>
									</Radio>
								))}
							</RadioGroup>
						)}
					</form.Field>
					<form.Subscribe selector={(s) => [s.values.priority, s.values.reportedAt] as const}>
						{([priority, reportedAt]) => {
							const sla = catalog.priorities.find((p) => p.priority === priority)?.slaMinutes;
							const from =
								mode === "agent" && reportedAt ? new Date(reportedAt).getTime() : Date.now();
							if (!sla || Number.isNaN(from)) return null;
							return (
								<p className="mt-3 text-sm">
									{t("Termin", "Deadline")}:{" "}
									<strong className="figures">
										{formatInstant(new Date(from + sla * 60_000).toISOString(), i18n)}
									</strong>
								</p>
							);
						}}
					</form.Subscribe>
				</Step>
			) : null}

			{mode === "agent" && at(2) ? (
				<Step title={t("Kiedy zgłoszono?", "When was it reported?")}>
					<form.Field name="reportedAt">
						{(field) => (
							<DateInput
								label={t("Data zgłoszenia", "Report date")}
								granularity="minute"
								value={field.state.value}
								onChange={field.handleChange}
								onBlur={field.handleBlur}
								minValue={localNow(-30)}
								maxValue={localNow()}
								errorMessage={
									field.state.meta.isTouched ? firstMessage(field.state.meta.errors) : undefined
								}
								description={t(
									"Telefon z rana albo mail z zeszłego tygodnia: najwyżej 30 dni wstecz.",
									"A call this morning or an e-mail last week: up to 30 days back.",
								)}
								className="max-w-xs"
							/>
						)}
					</form.Field>
				</Step>
			) : null}

			{at(3) ? (
				<Step title={t("Kto zgłasza?", "Who is asking?")}>
					<form.Field name="requesterPersonId">
						{(field) => (
							<form.Subscribe selector={(s) => s.values.requesterIncognito}>
								{(incognito) => (
									<div className="flex flex-col gap-3">
										<RequesterPicker
											people={people}
											value={field.state.value}
											isDisabled={incognito}
											onChange={field.handleChange}
										/>
										<form.Field name="requesterIncognito">
											{(inc) => (
												<Checkbox
													isSelected={inc.state.value}
													onChange={(on) => {
														inc.handleChange(on);
														if (on) field.handleChange(null);
													}}
												>
													<Checkbox.Content>
														<Checkbox.Control>
															<Checkbox.Indicator />
														</Checkbox.Control>
														{t("Incognito - nie podaję nazwiska", "Incognito - without my name")}
													</Checkbox.Content>
												</Checkbox>
											)}
										</form.Field>
										<FieldMessage
											errors={field.state.meta.isTouched ? field.state.meta.errors : []}
										/>
									</div>
								)}
							</form.Subscribe>
						)}
					</form.Field>
				</Step>
			) : null}

			{at(4) ? (
				<Step title={t("Załączniki (opcjonalnie)", "Attachments (optional)")}>
					<form.Subscribe selector={(st) => st.values.categoryKey}>
						{(categoryKey) => (
							<AttachmentPicker
								files={files}
								onChange={setFiles}
								extensions={catalog.attachmentExtensions}
								room={ATTACHMENTS_PER_TICKET}
								hint={attachmentHint(categoryKey, i18n)}
							/>
						)}
					</form.Subscribe>
				</Step>
			) : null}

			{at(4) ? (
				<form.Subscribe selector={(st) => st.values}>
					{(v) => {
						const category = catalog.categories.find((c) => c.key === v.categoryKey);
						const sub = category?.subcategories.find((x) => x.key === v.subcategoryKey);
						const person = people.find((x) => x.id === v.requesterPersonId);
						return (
							<Step title={t("Podsumowanie", "Summary")}>
								<dl className="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-1.5 rounded-md border border-separator bg-surface-secondary p-4 text-sm">
									<SummaryRow label={t("Temat", "Topic")}>
										{category ? n(category.name) : "-"}
										{sub ? ` / ${n(sub.name)}` : ""}
									</SummaryRow>
									<SummaryRow label={t("Tytuł", "Title")}>{v.title}</SummaryRow>
									<SummaryRow label={t("Priorytet", "Priority")}>
										{priorityLabel(v.priority, i18n)}
									</SummaryRow>
									<SummaryRow label={t("Zgłasza", "Requester")}>
										{v.requesterIncognito
											? t("Incognito", "Incognito")
											: person
												? `${person.firstName} ${person.lastName}`
												: "-"}
									</SummaryRow>
									{mode === "agent" ? (
										<SummaryRow label={t("Zgłoszono", "Reported")}>
											<span className="figures">
												{v.reportedAt
													? formatInstant(new Date(v.reportedAt).toISOString(), i18n)
													: "-"}
											</span>
										</SummaryRow>
									) : null}
								</dl>
							</Step>
						);
					}}
				</form.Subscribe>
			) : null}
		</>
	);

	const submitLabel =
		mode === "agent"
			? t("Zarejestruj zgłoszenie", "Register the ticket")
			: t("Wyślij zgłoszenie", "Send");
	const apiError = (
		<ApiError
			error={error}
			title={t("Nie udało się wysłać zgłoszenia", "The ticket could not be sent")}
		/>
	);

	const last = step === stages.length - 1;
	const body = (
		<div className="flex flex-col gap-6">
			<Stepper stages={stages} step={step} onPick={(i) => i < step && setStep(i)} />
			<div className="flex flex-col gap-6">{sections}</div>
			{last ? apiError : null}
		</div>
	);
	const buttons = (
		<>
			<span className="mr-auto font-mono text-xs text-muted">
				{t(`Krok ${step + 1} z ${stages.length}`, `Step ${step + 1} of ${stages.length}`)}
			</span>
			{step > 0 ? (
				<Button variant="secondary" onPress={() => setStep(step - 1)}>
					{t("Wstecz", "Back")}
				</Button>
			) : onCancel ? (
				<Button variant="secondary" onPress={onCancel}>
					{t("Anuluj", "Cancel")}
				</Button>
			) : null}
			{last ? (
				<Button type="submit" isPending={isPending}>
					<Check className="size-4" />
					{submitLabel}
				</Button>
			) : (
				<Button type="submit">{t("Dalej", "Next")}</Button>
			)}
		</>
	);

	return (
		<form
			noValidate
			className={frame === "dialog" ? "contents" : "flex flex-col"}
			onSubmit={(event) => {
				event.preventDefault();
				// Enter in a field moves on; only the last step sends.
				if (last) void form.handleSubmit();
				else next();
			}}
		>
			{frame === "dialog" ? (
				<>
					<Modal.Body>
						<div className="min-h-[420px] py-2">{body}</div>
					</Modal.Body>
					<Modal.Footer className="border-t border-separator pt-4">{buttons}</Modal.Footer>
				</>
			) : (
				<>
					<div className="min-h-[440px] px-5 py-6 sm:px-8">{body}</div>
					<div className="flex items-center gap-2 border-t border-separator bg-surface-secondary px-5 py-4 sm:px-8">
						{buttons}
					</div>
				</>
			)}
		</form>
	);
}

interface WizardStage {
	title: string;
	icon: LucideIcon;
	fields: readonly (keyof ReportValues)[];
}

/** Topic, the request, priority (and the report date for an agent), requester, attachments with a summary. */
function wizardStages({ t }: Pick<I18n, "t">, mode: "agent" | "public"): WizardStage[] {
	return [
		{ title: t("Temat", "Topic"), icon: Shapes, fields: ["categoryKey", "subcategoryKey"] },
		{ title: t("Opis", "Request"), icon: FileText, fields: ["title", "description"] },
		mode === "agent"
			? {
					title: t("Priorytet i data", "Priority and date"),
					icon: Gauge,
					fields: ["priority", "reportedAt"],
				}
			: { title: t("Priorytet", "Priority"), icon: Gauge, fields: ["priority"] },
		{ title: t("Zgłaszający", "Requester"), icon: UserRound, fields: ["requesterPersonId"] },
		{ title: t("Załączniki", "Attachments"), icon: Paperclip, fields: [] },
	];
}

function Stepper({
	stages,
	step,
	onPick,
}: {
	stages: WizardStage[];
	step: number;
	onPick: (i: number) => void;
}) {
	return (
		<ol className="flex items-start">
			{stages.map((st, i) => {
				const done = i < step;
				const current = i === step;
				const Icon = done ? Check : st.icon;
				return (
					<li key={st.title} className="flex flex-1 items-start last:flex-none">
						<button
							type="button"
							disabled={!done}
							onClick={() => onPick(i)}
							aria-current={current ? "step" : undefined}
							aria-label={st.title}
							className={`group flex flex-col items-center gap-1.5 ${done ? "cursor-pointer" : "cursor-default"}`}
						>
							<span
								className={`flex size-9 items-center justify-center rounded-full border-2 transition-colors ${
									done
										? "border-accent bg-accent text-accent-foreground group-hover:bg-accent-hover"
										: current
											? "border-accent bg-accent-soft text-accent shadow-[0_0_0_4px_var(--accent-soft)]"
											: "border-border bg-surface text-muted"
								}`}
							>
								<Icon className="size-4" strokeWidth={done ? 2.5 : 1.75} />
							</span>
							<span
								className={`hidden text-xs whitespace-nowrap sm:block ${
									current
										? "font-semibold text-foreground"
										: done
											? "text-foreground"
											: "text-muted"
								}`}
							>
								{st.title}
							</span>
						</button>
						{i < stages.length - 1 ? (
							<span
								aria-hidden
								className={`mx-2 mt-[18px] h-0.5 flex-1 rounded-full ${done ? "bg-accent" : "bg-border"}`}
							/>
						) : null}
					</li>
				);
			})}
		</ol>
	);
}

function SummaryRow({ label, children }: { label: string; children: ReactNode }) {
	return (
		<>
			<dt className="text-muted">{label}</dt>
			<dd className="min-w-0 truncate text-foreground">{children}</dd>
		</>
	);
}

function Step({ title, children }: { title: string; children: ReactNode }) {
	return (
		<section className="flex flex-col gap-3">
			<h2 className="font-display text-lg font-semibold text-foreground">{title}</h2>
			{children}
		</section>
	);
}

function firstMessage(errors: unknown[]): string | undefined {
	const first = errors.find(Boolean);
	if (!first) return undefined;
	return typeof first === "string" ? first : (first as { message?: string }).message;
}

function FieldMessage({ errors }: { errors: unknown[] }) {
	const message = firstMessage(errors);
	return message ? <p className="text-sm text-danger">{message}</p> : null;
}

/** Pick a person by name or department. Picking a name is not authentication (ADR-0007). */
function RequesterPicker({
	people,
	value,
	isDisabled,
	onChange,
}: {
	people: PublicPerson[];
	value: number | null;
	isDisabled: boolean;
	onChange: (id: number | null) => void;
}) {
	const { t, n } = useI18n();
	const [text, setText] = useState("");
	const q = text.trim().toLocaleLowerCase("pl");
	const shown = people.filter(
		(p) =>
			p.id === value ||
			q === "" ||
			`${p.firstName} ${p.lastName}`.toLocaleLowerCase("pl").includes(q) ||
			p.department.toLocaleLowerCase("pl").includes(q),
	);
	return (
		<div className={`flex flex-col gap-2 ${isDisabled ? "pointer-events-none opacity-50" : ""}`}>
			<SearchField
				aria-label={t("Szukaj osoby", "Search people")}
				value={text}
				onChange={setText}
				isDisabled={isDisabled}
				fullWidth
			>
				<SearchField.Group>
					<SearchField.SearchIcon />
					<SearchField.Input placeholder={t("Imię, nazwisko albo dział…", "Name or department…")} />
					<SearchField.ClearButton />
				</SearchField.Group>
			</SearchField>
			<ListBox
				aria-label={t("Osoby", "People")}
				selectionMode="single"
				className="max-h-56 overflow-y-auto rounded-md border border-separator"
				selectedKeys={value != null ? new Set([value]) : new Set()}
				onSelectionChange={(keys) => {
					const key = keys === "all" ? undefined : [...keys][0];
					onChange(key == null ? null : Number(key));
				}}
			>
				{shown.map((p) => (
					<ListBox.Item key={p.id} id={p.id} textValue={`${p.firstName} ${p.lastName}`}>
						<span className="flex w-full justify-between gap-3">
							<span>
								{p.firstName} {p.lastName}
							</span>
							<span className="text-xs text-muted">{n(p.department)}</span>
						</span>
						<ListBox.ItemIndicator />
					</ListBox.Item>
				))}
			</ListBox>
		</div>
	);
}
