import { createContext, type ReactNode, useCallback, useContext, useMemo, useState } from "react";
import { catalogEn } from "./catalogEn";

/*
 * Polish and English (ADR-0008). Polish is the default: the team works in Polish whatever the browser says, so the
 * language is the user's choice only (kept in this browser), never Accept-Language. Texts are written in both
 * languages where they are used, `t("Pula", "Pool")`: no keys and no resource files, a page reads with its English
 * next to the Polish. Stored data (catalog names, what people typed) never changes; `n()` translates catalog names
 * for display.
 */
export type Lang = "pl" | "en";

const STORAGE_KEY = "crm.lang";

function initialLang(): Lang {
	try {
		return window.localStorage.getItem(STORAGE_KEY) === "en" ? "en" : "pl";
	} catch {
		return "pl";
	}
}

interface LangState {
	lang: Lang;
	setLang: (lang: Lang) => void;
}

const LangContext = createContext<LangState | null>(null);

export function LanguageProvider({ children }: { children: ReactNode }) {
	const [lang, setLangState] = useState<Lang>(() => {
		const value = initialLang();
		document.documentElement.lang = value;
		return value;
	});
	const setLang = useCallback((value: Lang) => {
		try {
			window.localStorage.setItem(STORAGE_KEY, value);
		} catch {
			// Private mode or blocked storage: the choice lasts until the page is reloaded.
		}
		document.documentElement.lang = value;
		setLangState(value);
	}, []);
	const state = useMemo(() => ({ lang, setLang }), [lang, setLang]);
	return <LangContext.Provider value={state}>{children}</LangContext.Provider>;
}

export interface I18n {
	lang: Lang;
	setLang: (lang: Lang) => void;
	/** The text in the current language. */
	t: (pl: string, en: string) => string;
	/** A catalog name (category, subcategory, market, department, field, choice) for display. */
	n: (name: string | null | undefined) => string;
	/** BCP 47 locale for Intl formatting. */
	locale: "pl-PL" | "en-GB";
}

export function useI18n(): I18n {
	const state = useContext(LangContext);
	if (!state) throw new Error("useI18n outside LanguageProvider");
	const { lang, setLang } = state;
	return useMemo(
		() => ({
			lang,
			setLang,
			t: (pl: string, en: string) => (lang === "pl" ? pl : en),
			n: (name: string | null | undefined) =>
				name ? (lang === "en" ? (catalogEn[name] ?? name) : name) : "",
			locale: lang === "pl" ? "pl-PL" : "en-GB",
		}),
		[lang, setLang],
	);
}

/** Times are stored in UTC and shown in Warsaw time; deadlines are absolute, so a DST change does not move them. */
export const TIME_ZONE = "Europe/Warsaw";
