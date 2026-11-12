import { Button } from "@heroui/react";
import type { OnDemandTotal } from "#/hooks/useOnDemandTotal";
import { useI18n } from "#/i18n";

/**
 * Keyset pages: a page says only "is there a next page". The total is a separate, explicit request (`total`),
 * because counting reads every matching row; lists without a count endpoint leave it out.
 */
export function LoadMore({
	shown,
	hasMore,
	isLoading,
	onLoadMore,
	noun,
	total,
}: {
	shown: number;
	hasMore: boolean;
	isLoading: boolean;
	onLoadMore: () => void;
	noun: string;
	total?: OnDemandTotal;
}) {
	const { t, locale } = useI18n();
	const known = total?.total;
	return (
		<div className="flex items-center justify-between gap-3 border-t border-separator px-4 py-3">
			<div className="flex flex-wrap items-center gap-2">
				<span className="font-mono text-xs text-muted">
					{shown.toLocaleString(locale)}
					{hasMore && known !== undefined ? ` ${t("z", "of")} ${known.toLocaleString(locale)}` : ""}{" "}
					{noun}
					{hasMore && known === undefined ? ` ${t("na liście", "shown")}` : ""}
				</span>
				{/* On the last page everything is shown, so the count is known without asking the database. */}
				{hasMore && total && known === undefined && (
					<Button size="sm" variant="ghost" isPending={total.isLoading} onPress={total.request}>
						{total.isError
							? t("Policz ponownie", "Retry total")
							: t("Policz wszystkie", "Show total")}
					</Button>
				)}
				{hasMore && total?.isError && (
					<span className="text-xs text-danger">
						{t("Nie udało się policzyć wszystkich.", "The total could not be counted.")}
					</span>
				)}
			</div>
			{hasMore ? (
				<Button size="sm" variant="secondary" isPending={isLoading} onPress={onLoadMore}>
					{t("Pokaż więcej", "Load more")}
				</Button>
			) : (
				<span className="text-xs text-muted">{t("Koniec listy", "End of list")}</span>
			)}
		</div>
	);
}
