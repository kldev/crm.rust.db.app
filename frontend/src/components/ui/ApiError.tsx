import { Alert } from "@heroui/react";
import { useI18n } from "#/i18n";
import { problemText } from "#/i18n/problems";

/** The message of a failed request (the procedure's refusal, in the user's language), or nothing. */
export function ApiError({ error, title }: { error: unknown; title?: string }) {
	const i18n = useI18n();
	if (!error) return null;
	return (
		<Alert status="danger">
			<Alert.Indicator />
			<Alert.Content>
				<Alert.Title>{title ?? i18n.t("To się nie udało", "That did not work")}</Alert.Title>
				<Alert.Description>{problemText(error, i18n)}</Alert.Description>
			</Alert.Content>
		</Alert>
	);
}
