import { Calendar, DateField, DatePicker, Description, FieldError, Label } from "@heroui/react";
import {
	type CalendarDate,
	type CalendarDateTime,
	parseDate,
	parseDateTime,
} from "@internationalized/date";
import { I18nProvider } from "react-aria-components";
import { useI18n } from "#/i18n";

type Granularity = "day" | "minute";

/** "YYYY-MM-DD" (day) or "YYYY-MM-DDTHH:mm" (minute); "" or a broken value is no date. */
function parse(value: string | undefined, granularity: Granularity) {
	if (!value) return null;
	try {
		return granularity === "day"
			? parseDate(value.slice(0, 10))
			: parseDateTime(value.slice(0, 16));
	} catch {
		return null;
	}
}

/** Back to the string the forms and the URL keep: the same shapes `<input type="date|datetime-local">` used. */
function format(value: CalendarDate | CalendarDateTime | null, granularity: Granularity) {
	if (!value) return "";
	return granularity === "day" ? value.toString().slice(0, 10) : value.toString().slice(0, 16);
}

/**
 * A HeroUI date picker over a plain string value: typed segments in the user's language (pl-PL / en-GB) and a
 * calendar in a popover. granularity "minute" adds the time, for a report taken by phone this morning.
 */
export function DateInput({
	label,
	value,
	onChange,
	onBlur,
	granularity = "day",
	minValue,
	maxValue,
	description,
	errorMessage,
	isRequired,
	className,
	fullWidth,
}: {
	label: string;
	value: string | undefined;
	onChange: (value: string) => void;
	onBlur?: () => void;
	granularity?: Granularity;
	minValue?: string;
	maxValue?: string;
	description?: string;
	/** Shown and marks the field invalid when set. */
	errorMessage?: string;
	isRequired?: boolean;
	className?: string;
	fullWidth?: boolean;
}) {
	const { locale, t } = useI18n();
	return (
		<I18nProvider locale={locale}>
			<DatePicker
				className={className}
				value={parse(value, granularity)}
				onChange={(v) => onChange(format(v, granularity))}
				onBlur={onBlur}
				granularity={granularity}
				hourCycle={24}
				minValue={parse(minValue, granularity) ?? undefined}
				maxValue={parse(maxValue, granularity) ?? undefined}
				isInvalid={Boolean(errorMessage)}
				isRequired={isRequired}
				// zod is the authority; native validation would add a second message
				validationBehavior="aria"
			>
				<Label>{label}</Label>
				<DateField.Group fullWidth={fullWidth}>
					<DateField.InputContainer>
						<DateField.Input>
							{(segment) => <DateField.Segment segment={segment} />}
						</DateField.Input>
					</DateField.InputContainer>
					<DateField.Suffix>
						<DatePicker.Trigger aria-label={t("Otwórz kalendarz", "Open calendar")}>
							<DatePicker.TriggerIndicator />
						</DatePicker.Trigger>
					</DateField.Suffix>
				</DateField.Group>
				{description ? <Description>{description}</Description> : null}
				<FieldError>{errorMessage}</FieldError>
				<DatePicker.Popover>
					<Calendar aria-label={label}>
						<Calendar.Header>
							<Calendar.Heading />
							<Calendar.NavButton slot="previous" />
							<Calendar.NavButton slot="next" />
						</Calendar.Header>
						<Calendar.Grid>
							<Calendar.GridHeader>
								{(day) => <Calendar.HeaderCell>{day}</Calendar.HeaderCell>}
							</Calendar.GridHeader>
							<Calendar.GridBody>{(date) => <Calendar.Cell date={date} />}</Calendar.GridBody>
						</Calendar.Grid>
					</Calendar>
				</DatePicker.Popover>
			</DatePicker>
		</I18nProvider>
	);
}
