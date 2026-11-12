/** Formatting shared by the pages. The API sends UTC timestamps without a zone (datetime2) and ISO dates. */

const dateFmt = new Intl.DateTimeFormat("en-GB", {
	day: "2-digit",
	month: "short",
	year: "numeric",
});
const dateTimeFmt = new Intl.DateTimeFormat("en-GB", {
	day: "2-digit",
	month: "short",
	year: "numeric",
	hour: "2-digit",
	minute: "2-digit",
});
const moneyFmt = new Intl.NumberFormat("pl-PL", {
	style: "currency",
	currency: "PLN",
	maximumFractionDigits: 0,
});
const relativeFmt = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

const utc = (value: string) => new Date(value.endsWith("Z") ? value : `${value}Z`);

export const formatDate = (isoDate: string) => dateFmt.format(new Date(`${isoDate}T00:00:00`));
export const formatDateTime = (timestamp: string) => dateTimeFmt.format(utc(timestamp));
export const formatMoney = (amount: number) => moneyFmt.format(amount);

export function formatRelative(timestamp: string): string {
	const seconds = (utc(timestamp).getTime() - Date.now()) / 1000;
	const steps: [Intl.RelativeTimeFormatUnit, number][] = [
		["year", 31_536_000],
		["month", 2_592_000],
		["week", 604_800],
		["day", 86_400],
		["hour", 3_600],
		["minute", 60],
	];
	for (const [unit, size] of steps) {
		if (Math.abs(seconds) >= size) return relativeFmt.format(Math.round(seconds / size), unit);
	}
	return "just now";
}

/** Days from today to an ISO date (negative = in the past). */
export function daysUntil(isoDate: string): number {
	const today = new Date();
	today.setHours(0, 0, 0, 0);
	return Math.round((new Date(`${isoDate}T00:00:00`).getTime() - today.getTime()) / 86_400_000);
}

export const todayIso = () => new Date().toISOString().slice(0, 10);
