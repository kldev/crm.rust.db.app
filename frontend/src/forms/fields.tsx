import {
	Description,
	FieldError,
	Input,
	Label,
	ListBox,
	Select,
	TextArea,
	TextField,
} from "@heroui/react";
import { useFieldContext } from "./context";

/** TanStack Form keeps zod issues (or strings); HeroUI shows one message. */
function firstError(errors: unknown[]): string | undefined {
	const first = errors.find(Boolean);
	if (!first) return undefined;
	return typeof first === "string" ? first : (first as { message?: string }).message;
}

function useFieldState<T>() {
	const field = useFieldContext<T>();
	const message = field.state.meta.isTouched ? firstError(field.state.meta.errors) : undefined;
	return { field, message };
}

export function FormInput({
	label,
	type = "text",
	placeholder,
	description,
	autoFocus,
	autoComplete,
	inputMode,
}: {
	label: string;
	type?: "text" | "email" | "password" | "date" | "tel";
	placeholder?: string;
	description?: string;
	autoFocus?: boolean;
	autoComplete?: string;
	inputMode?: "text" | "decimal" | "numeric" | "email" | "tel";
}) {
	const { field, message } = useFieldState<string>();
	return (
		<TextField
			fullWidth
			name={field.name}
			type={type}
			value={field.state.value ?? ""}
			onChange={field.handleChange}
			onBlur={field.handleBlur}
			isInvalid={Boolean(message)}
			// zod is the authority; native validation would add a second message
			validationBehavior="aria"
			autoFocus={autoFocus}
		>
			<Label>{label}</Label>
			<Input placeholder={placeholder} autoComplete={autoComplete} inputMode={inputMode} />
			{description ? <Description>{description}</Description> : null}
			<FieldError>{message}</FieldError>
		</TextField>
	);
}

export function FormTextArea({
	label,
	placeholder,
	rows = 4,
}: {
	label: string;
	placeholder?: string;
	rows?: number;
}) {
	const { field, message } = useFieldState<string>();
	return (
		<TextField
			fullWidth
			name={field.name}
			value={field.state.value ?? ""}
			onChange={field.handleChange}
			onBlur={field.handleBlur}
			isInvalid={Boolean(message)}
			validationBehavior="aria"
		>
			<Label>{label}</Label>
			<TextArea placeholder={placeholder} rows={rows} />
			<FieldError>{message}</FieldError>
		</TextField>
	);
}

export interface SelectOption {
	id: string;
	label: string;
	description?: string;
}

export function FormSelect({
	label,
	options,
	placeholder = "Select…",
	description,
	isDisabled,
}: {
	label: string;
	options: SelectOption[];
	placeholder?: string;
	description?: string;
	isDisabled?: boolean;
}) {
	const { field, message } = useFieldState<string>();
	return (
		<Select
			fullWidth
			name={field.name}
			placeholder={placeholder}
			// "" in the form means nothing chosen; null shows the placeholder
			value={field.state.value || null}
			onChange={(key) => field.handleChange(key == null ? "" : String(key))}
			onBlur={field.handleBlur}
			isInvalid={Boolean(message)}
			isDisabled={isDisabled}
			validationBehavior="aria"
		>
			<Label>{label}</Label>
			<Select.Trigger>
				<Select.Value />
				<Select.Indicator />
			</Select.Trigger>
			{description ? <Description>{description}</Description> : null}
			<FieldError>{message}</FieldError>
			<Select.Popover>
				<ListBox>
					{options.map((option) => (
						<ListBox.Item key={option.id} id={option.id} textValue={option.label}>
							<div className="flex flex-col">
								<span>{option.label}</span>
								{option.description ? (
									<span className="text-xs text-muted">{option.description}</span>
								) : null}
							</div>
							<ListBox.ItemIndicator />
						</ListBox.Item>
					))}
				</ListBox>
			</Select.Popover>
		</Select>
	);
}
