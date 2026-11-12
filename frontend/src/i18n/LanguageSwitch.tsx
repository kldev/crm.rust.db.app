import { ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { type Lang, useI18n } from ".";

/** PL / EN switch: in the top bar, on the sign-in page and on the public form. */
export function LanguageSwitch({ className }: { className?: string }) {
	const { lang, setLang, t } = useI18n();
	return (
		<ToggleButtonGroup
			aria-label={t("Język", "Language")}
			selectionMode="single"
			disallowEmptySelection
			size="sm"
			className={className}
			selectedKeys={new Set([lang])}
			onSelectionChange={(keys) => {
				const next = [...keys][0];
				if (next === "pl" || next === "en") setLang(next as Lang);
			}}
		>
			<ToggleButton id="pl" aria-label="Polski">
				PL
			</ToggleButton>
			<ToggleButton id="en" aria-label="English">
				EN
			</ToggleButton>
		</ToggleButtonGroup>
	);
}
