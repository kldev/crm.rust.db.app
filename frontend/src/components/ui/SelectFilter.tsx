import { Label, ListBox, Select } from "@heroui/react";
import { useI18n } from "#/i18n";

const ANY = "__any";

/** A toolbar filter: "Any" clears it. The value lives in the URL, so it is a string or undefined. */
export function SelectFilter({
	label,
	value,
	options,
	onChange,
	className = "w-full sm:w-44",
}: {
	label: string;
	value: string | undefined;
	options: { id: string; label: string }[];
	onChange: (value: string | undefined) => void;
	className?: string;
}) {
	const { t } = useI18n();
	const any = t("Dowolne", "Any");
	return (
		<Select
			className={className}
			value={value ?? ANY}
			onChange={(key) => onChange(key == null || key === ANY ? undefined : String(key))}
		>
			<Label className="sr-only">{label}</Label>
			<Select.Trigger>
				<span className="mr-1 text-xs text-muted">{label}:</span>
				<Select.Value />
				<Select.Indicator />
			</Select.Trigger>
			<Select.Popover>
				<ListBox>
					<ListBox.Item id={ANY} textValue={any}>
						{any}
						<ListBox.ItemIndicator />
					</ListBox.Item>
					{options.map((o) => (
						<ListBox.Item key={o.id} id={o.id} textValue={o.label}>
							{o.label}
							<ListBox.ItemIndicator />
						</ListBox.Item>
					))}
				</ListBox>
			</Select.Popover>
		</Select>
	);
}
