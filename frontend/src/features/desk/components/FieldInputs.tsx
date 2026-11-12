import {
	Checkbox,
	CheckboxGroup,
	Description,
	Header,
	Input,
	Label,
	ListBox,
	Select,
	TextField,
} from "@heroui/react";
import type { FieldDefinitionResponse } from "#/api/models";
import { useI18n } from "#/i18n";

export type FieldValues = Record<string, string>;

const NONE = "__none";

/**
 * The report data of a subcategory (ADR-0010): one control per field, chosen by its kind. Values are the strings the
 * procedure checks and stores (dates yyyy-mm-dd, multi-choice "A, B"); nothing is validated here but the obvious -
 * the procedure decides and its refusal names the field. Required fields carry an asterisk.
 */
export function FieldInputs({
	fields,
	values,
	onChange,
}: {
	fields: FieldDefinitionResponse[];
	values: FieldValues;
	onChange: (values: FieldValues) => void;
}) {
	const set = (key: string, value: string) => onChange({ ...values, [key]: value });
	return (
		<div className="grid gap-4 sm:grid-cols-2">
			{fields.map((field) => (
				<div
					key={field.key}
					className={field.kind === "MULTI_CHOICE" ? "sm:col-span-2" : undefined}
				>
					<FieldInput
						field={field}
						value={values[field.key] ?? ""}
						onChange={(v) => set(field.key, v)}
					/>
				</div>
			))}
		</div>
	);
}

function FieldInput({
	field,
	value,
	onChange,
}: {
	field: FieldDefinitionResponse;
	value: string;
	onChange: (value: string) => void;
}) {
	const { t, n } = useI18n();
	const label = `${n(field.label)}${field.required ? " *" : ""}`;

	if (field.kind === "CHOICE") {
		// Favourites first (a country list starts with the usual markets), then the rest in list order.
		const rest = field.options.filter((o) => !field.favorites.includes(o));
		const item = (o: string) => (
			<ListBox.Item key={o} id={o} textValue={n(o)}>
				{n(o)}
				<ListBox.ItemIndicator />
			</ListBox.Item>
		);
		return (
			<Select
				fullWidth
				value={value || NONE}
				onChange={(key) => onChange(key == null || key === NONE ? "" : String(key))}
			>
				<Label>{label}</Label>
				<Select.Trigger>
					<Select.Value />
					<Select.Indicator />
				</Select.Trigger>
				<Select.Popover>
					<ListBox>
						<ListBox.Item id={NONE} textValue="—">
							—
							<ListBox.ItemIndicator />
						</ListBox.Item>
						{field.favorites.length > 0 ? (
							<>
								<ListBox.Section>
									<Header>{t("Najczęstsze", "Most used")}</Header>
									{field.favorites.map(item)}
								</ListBox.Section>
								<ListBox.Section>
									<Header>{t("Pozostałe", "Others")}</Header>
									{rest.map(item)}
								</ListBox.Section>
							</>
						) : (
							rest.map(item)
						)}
					</ListBox>
				</Select.Popover>
			</Select>
		);
	}

	if (field.kind === "MULTI_CHOICE") {
		const picked = value
			.split(",")
			.map((v) => v.trim())
			.filter(Boolean);
		return (
			<CheckboxGroup
				value={picked}
				onChange={(next) => onChange(field.options.filter((o) => next.includes(o)).join(", "))}
			>
				<Label>{label}</Label>
				<div className="flex flex-wrap gap-x-5 gap-y-2">
					{field.options.map((o) => (
						<Checkbox key={o} value={o}>
							<Checkbox.Content>
								<Checkbox.Control>
									<Checkbox.Indicator />
								</Checkbox.Control>
								{n(o)}
							</Checkbox.Content>
						</Checkbox>
					))}
				</div>
			</CheckboxGroup>
		);
	}

	const input = {
		DATE: { type: "date" as const },
		EMAIL: { type: "email" as const },
		AMOUNT: { inputMode: "decimal" as const, placeholder: "1 234,56" },
		COUNT: { inputMode: "numeric" as const },
		IBAN: { placeholder: "PL00 0000 0000 0000 0000 0000 0000" },
		TICKET_NUMBER: { placeholder: "HAL-1A2B3C4D" },
	}[field.kind] ?? { type: "text" as const };

	return (
		<TextField
			fullWidth
			value={value}
			onChange={onChange}
			type={"type" in input ? input.type : "text"}
		>
			<Label>{label}</Label>
			<Input
				inputMode={"inputMode" in input ? input.inputMode : undefined}
				placeholder={field.placeholder ?? ("placeholder" in input ? input.placeholder : undefined)}
			/>
			{field.kind === "AMOUNT" ? (
				<Description>{t("Najwyżej 2 miejsca po przecinku", "At most 2 decimals")}</Description>
			) : null}
		</TextField>
	);
}
