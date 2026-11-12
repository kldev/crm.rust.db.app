import { useMemo } from "react";
import type { DeskCatalog } from "#/api/models";
import { useI18n } from "#/i18n";

/** Display names of stored catalog keys, in the user's language; an unknown key shows as it is. */
export function useCatalogNames(catalog: DeskCatalog | undefined) {
	const { n } = useI18n();
	return useMemo(() => {
		const categories = new Map(catalog?.categories.map((c) => [c.key, c]) ?? []);
		const subs = new Map(
			catalog?.categories.flatMap((c) => c.subcategories.map((s) => [s.key, s] as const)) ?? [],
		);
		return {
			category: (key: string) => n(categories.get(key)?.name) || key,
			/** "Belgia · Umowa i zatrudnienie" when the subcategory has a market. */
			subcategory: (key: string) => {
				const s = subs.get(key);
				if (!s) return key;
				return s.market ? `${n(s.market)} · ${n(s.name)}` : n(s.name);
			},
			department: (key: string) => categories.get(key)?.department ?? null,
		};
	}, [catalog, n]);
}
