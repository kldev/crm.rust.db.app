import {
	Button,
	Checkbox,
	Description,
	FieldError,
	Input,
	Label,
	ListBox,
	Modal,
	Radio,
	RadioGroup,
	SearchField,
	TextField,
} from "@heroui/react";
import { Check } from "lucide-react";
import { type ReactNode, useMemo, useState } from "react";
import { z } from "zod";
import type { DeskCatalog, PublicPerson } from "#/api/models";
import { ApiError } from "#/components/ui";
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
	/** `datetime-local` value (the browser's time zone); only on the agent's form. */
	reportedAt: string;
}

/** A `datetime-local` value for now, to the minute. */
export function localNow(): string {
	const d = new Date();
	d.setSeconds(0, 0);
	return new Date(d.getTime() - d.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
}

/**
 * The report form (ADR-0007), the same for the agent and for the public page: category tiles, then the subcategory
 * (grouped by market), title and description, priority with its SLA and the deadline before sending, and the
 * requester - a person from the list or incognito. The agent's form also takes the report date (up to 30 days back).
 * zod checks what the form can know; the procedure is the authority and its refusal shows under the form.
 *
 * variant "wizard" (the signed-in agent's dialog) shows the same sections a step at a time, inside a Modal.Dialog:
 * "Next" checks only the step's fields, so a missing title never blocks choosing the topic.
 */
export function ReportForm({
	catalog,
	people,
	mode,
	isPending,
	error,
	onSubmit,
	initial,
	variant = "page",
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
	/** page: every section on one page; wizard: a step at a time, for a Modal.Dialog. */
	variant?: "page" | "wizard";
	/** wizard: "Cancel" on the first step. */
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
	const wizard = variant === "wizard";
	const [step, setStep] = useState(0);
	const stages = wizardStages(i18n);
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
	const at = (i: number) => !wizard || step === i;
	const num = (n: number) => (wizard ? undefined : n);

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
		// Wizard: a refusal sends the agent back to the first step with a wrong field.
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
				<Step n={num(1)} title={t("Czego dotyczy sprawa?", "What is it about?")}>
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
												className={`flex flex-col items-start gap-2 rounded-xl border p-4 text-left transition-colors ${
													chosen
														? "border-accent bg-accent-soft ring-1 ring-accent"
														: "border-separator bg-surface hover:bg-surface-secondary"
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
							<Step n={num(2)} title={t("Temat", "Topic")}>
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
				<Step n={num(3)} title={t("Opis sprawy", "The request")}>
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
				<Step n={num(4)} title={t("Priorytet", "Priority")}>
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
				<Step n={num(7)} title={t("Kiedy zgłoszono?", "When was it reported?")}>
					<form.Field name="reportedAt">
						{(field) => (
							<TextField
								type="datetime-local"
								value={field.state.value}
								onChange={field.handleChange}
								onBlur={field.handleBlur}
								validationBehavior="aria"
								isInvalid={field.state.meta.isTouched && field.state.meta.errors.length > 0}
								className="max-w-xs"
							>
								<Label>{t("Data zgłoszenia", "Report date")}</Label>
								<Input max={localNow()} />
								<Description>
									{t(
										"Telefon z rana albo mail z zeszłego tygodnia: najwyżej 30 dni wstecz.",
										"A call this morning or an e-mail last week: up to 30 days back.",
									)}
								</Description>
								<FieldError>{firstMessage(field.state.meta.errors)}</FieldError>
							</TextField>
						)}
					</form.Field>
				</Step>
			) : null}

			{at(3) ? (
				<Step n={num(5)} title={t("Kto zgłasza?", "Who is asking?")}>
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
				<Step n={num(6)} title={t("Załączniki (opcjonalnie)", "Attachments (optional)")}>
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

			{wizard && at(4) ? (
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

	if (wizard) {
		const last = step === stages.length - 1;
		return (
			<form
				noValidate
				className="contents"
				onSubmit={(event) => {
					event.preventDefault();
					// Enter in a field moves on; only the last step sends.
					if (last) void form.handleSubmit();
					else next();
				}}
			>
				<Modal.Body>
					<div className="flex min-h-[420px] flex-col gap-6 py-2">
						<Stepper stages={stages} step={step} onPick={(i) => i < step && setStep(i)} />
						{sections}
						{last ? apiError : null}
					</div>
				</Modal.Body>
				<Modal.Footer className="border-t border-separator pt-4">
					<span className="mr-auto font-mono text-xs text-muted">
						{t(`Krok ${step + 1} z ${stages.length}`, `Step ${step + 1} of ${stages.length}`)}
					</span>
					{step === 0 ? (
						<Button variant="secondary" onPress={onCancel}>
							{t("Anuluj", "Cancel")}
						</Button>
					) : (
						<Button variant="secondary" onPress={() => setStep(step - 1)}>
							{t("Wstecz", "Back")}
						</Button>
					)}
					{last ? (
						<Button type="submit" isPending={isPending}>
							{submitLabel}
						</Button>
					) : (
						<Button type="submit">{t("Dalej", "Next")}</Button>
					)}
				</Modal.Footer>
			</form>
		);
	}

	return (
		<form
			noValidate
			className="flex flex-col gap-8"
			onSubmit={(event) => {
				event.preventDefault();
				void form.handleSubmit();
			}}
		>
			{sections}
			{apiError}
			<div className="flex justify-end">
				<Button type="submit" size="lg" isPending={isPending}>
					{submitLabel}
				</Button>
			</div>
		</form>
	);
}

interface WizardStage {
	title: string;
	fields: readonly (keyof ReportValues)[];
}

/** The agent's dialog: topic, the request, priority and date, requester, attachments with a summary. */
function wizardStages({ t }: Pick<I18n, "t">): WizardStage[] {
	return [
		{ title: t("Temat", "Topic"), fields: ["categoryKey", "subcategoryKey"] },
		{ title: t("Opis", "Request"), fields: ["title", "description"] },
		{ title: t("Priorytet i data", "Priority and date"), fields: ["priority", "reportedAt"] },
		{ title: t("Zgłaszający", "Requester"), fields: ["requesterPersonId"] },
		{ title: t("Załączniki", "Attachments"), fields: [] },
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
		<ol className="flex flex-wrap items-center gap-x-2 gap-y-2 border-b border-separator pb-4">
			{stages.map((st, i) => {
				const done = i < step;
				const current = i === step;
				return (
					<li key={st.title} className="flex items-center gap-2">
						<button
							type="button"
							disabled={!done}
							onClick={() => onPick(i)}
							aria-current={current ? "step" : undefined}
							className={`flex items-center gap-2 rounded px-1.5 py-1 text-sm ${
								done ? "cursor-pointer text-foreground hover:bg-default" : "cursor-default"
							} ${current ? "font-semibold text-foreground" : done ? "" : "text-muted"}`}
						>
							<span
								className={`flex size-6 items-center justify-center rounded-full font-mono text-[11px] ${
									done
										? "bg-accent text-accent-foreground"
										: current
											? "bg-ink text-white"
											: "border border-border bg-surface"
								}`}
							>
								{done ? <Check className="size-3.5" strokeWidth={2.5} /> : i + 1}
							</span>
							{st.title}
						</button>
						{i < stages.length - 1 ? <span className="h-px w-6 bg-border" aria-hidden /> : null}
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

function Step({ n, title, children }: { n?: number; title: string; children: ReactNode }) {
	return (
		<section className="flex flex-col gap-3">
			<h2 className="flex items-center gap-3 font-display text-xl font-semibold text-foreground">
				{n ? (
					<span className="flex size-7 items-center justify-center rounded-full bg-accent-soft font-mono text-xs text-ink">
						{n}
					</span>
				) : null}
				{title}
			</h2>
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
