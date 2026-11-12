import axios from "axios";
import { type Problem, problemMessage } from "#/api/mutator";
import type { I18n } from ".";

/*
 * Refusals of the procedures in the user's language (ADR-0008). The database throws a number and an English
 * message; the number picks the text here. A message with a name or a number in it is matched by a pattern and the
 * captured part (a person's name, a department) goes into the Polish text. A number this table does not know shows
 * the server's message as it is.
 */
type Text = { pl: string; en?: string; match?: RegExp };

const DESK: Record<number, Text> = {
	51100: { pl: "Pole „Tytuł” jest wymagane." },
	51101: { pl: "Pole „Tytuł” może mieć najwyżej 200 znaków." },
	51102: { pl: "Pole „Opis” jest wymagane." },
	51103: { pl: "Pole „Opis” może mieć najwyżej 4000 znaków." },
	51104: { pl: "Wybierz kategorię z listy." },
	51105: { pl: "Wybierz podkategorię należącą do wybranej kategorii." },
	51106: { pl: "Podkategoria „$1” nie przyjmuje już nowych zgłoszeń.", match: /"(.+)"/ },
	51107: { pl: "Wybierz priorytet." },
	51108: { pl: "Data zgłoszenia nie może być w przyszłości." },
	51109: { pl: "Data zgłoszenia może być najwyżej 30 dni wstecz." },
	51110: { pl: "Wybierz zgłaszającego z listy albo zaznacz „incognito”." },
	51111: { pl: "Pole „Powód” może mieć najwyżej 500 znaków." },
	51112: { pl: "Pole „Komentarz” jest wymagane." },
	51113: { pl: "Pole „Komentarz” może mieć najwyżej 2000 znaków." },
	51114: { pl: "Nie można przekazać zgłoszenia samemu sobie." },
	51115: { pl: "Wybierz osobę z działu $1.", match: /department (.+)\.$/ },
	51116: { pl: "Nieprawidłowy filtr listy." },
	51120: { pl: "Plik nie ma nazwy." },
	51121: {
		pl: "Plik „$1” ma niedozwolony typ. Dozwolone: obrazy (PNG, JPG, GIF, WEBP), PDF, dokumenty Office, TXT/CSV, ZIP, MP4/MOV.",
		match: /"(.+)"/,
	},
	51122: { pl: "Plik „$1” jest pusty.", match: /"(.+)"/ },
	51123: { pl: "Plik „$1” jest za duży. Limit to 20 MB.", match: /"(.+)"/ },
	51124: { pl: "Zgłoszenie może mieć najwyżej 10 załączników." },
	51130: { pl: "Pole „$1” nie należy do tej podkategorii.", match: /"(.+)"/ },
	51131: { pl: "Uzupełnij pole „$1”.", match: /"(.+)"/ },
	51132: { pl: "Pole „$1” może mieć najwyżej 200 znaków.", match: /"(.+)"/ },
	51133: { pl: "Pole „$1” musi być datą (rrrr-mm-dd).", match: /"(.+)"/ },
	51134: { pl: "Pole „$1” musi być adresem e-mail.", match: /"(.+)"/ },
	51135: {
		pl: "Pole „$1” musi być kwotą, np. 1 234,56 (najwyżej 2 miejsca po przecinku).",
		match: /"(.+)"/,
	},
	51136: { pl: "Pole „$1” nie jest poprawnym numerem rachunku (IBAN).", match: /"(.+)"/ },
	51137: { pl: "Pole „$1” musi być liczbą od 1 do 10000.", match: /"(.+)"/ },
	51138: { pl: "Wartości „$1” nie ma na liście tego pola.", match: /^"([^"]+)"/ },
	51139: { pl: "Wybierz wartość pola „$1” z listy.", match: /"(.+)"/ },
	51140: { pl: "Pole „$1” musi być numerem zgłoszenia, np. HAL-1A2B3C4D.", match: /"(.+)"/ },
	51141: { pl: "Wybierz rodzaj dokumentu, do którego należy numer." },
	51142: { pl: "Przy „Brak dokumentów” nie podaje się numeru dokumentu." },
	51143: { pl: "Podaj numer PESEL." },
	51144: {
		pl: "Numer PESEL jest niepoprawny (11 cyfr: prawdziwa data urodzenia i poprawna cyfra kontrolna).",
	},
	51145: { pl: "Podaj numer dowodu osobistego." },
	51146: { pl: "Ta podkategoria nie ma danych do raportu." },
	51150: { pl: "Wybierz okres najwyżej jednego roku (od <= do)." },
	51310: { pl: "Tylko agent realizujący zgłoszenie ($1) może je przekazać.", match: /\((.+)\)/ },
	51311: {
		pl: "Tylko agent realizujący zgłoszenie ($1) może je oddać do puli.",
		match: /\((.+)\)/,
	},
	51312: {
		pl: "Tylko agent realizujący zgłoszenie ($1) może je oznaczyć jako zrealizowane.",
		match: /\((.+)\)/,
	},
	51313: {
		pl: "Tylko agent realizujący zgłoszenie ($1) może wskazać osobę realizującą.",
		match: /\((.+)\)/,
	},
	51410: { pl: "Nie znaleziono zgłoszenia." },
	51411: { pl: "Nie znaleziono osoby." },
	51412: { pl: "Nie znaleziono agenta albo jego konto jest nieaktywne." },
	51413: { pl: "Nie znaleziono załącznika." },
	51910: {
		pl: "Ktoś był szybszy: zgłoszenie jest już w realizacji u: $1.",
		en: "Someone was faster: the ticket is already in progress with $1.",
		match: /with (.+)\.$/,
	},
	51911: { pl: "Zgłoszenie jest już zrealizowane." },
	51912: { pl: "Zgłoszenie jest w puli - najpierw trzeba je przejąć." },
	51913: { pl: "Ktoś był szybszy: zgłoszenie właśnie się zmieniło. Odśwież i spróbuj ponownie." },
};

/** The message of a failed request in the user's language. */
export function problemText(error: unknown, i18n: Pick<I18n, "lang" | "t" | "n">): string {
	if (axios.isAxiosError<Problem>(error)) {
		const data = error.response?.data;
		if (!error.response)
			return i18n.t(
				"Brak połączenia z serwerem. Sprawdź połączenie i spróbuj ponownie.",
				"The server cannot be reached. Check your connection and try again.",
			);
		if (error.response.status === 429)
			return i18n.t(
				"Zbyt wiele zgłoszeń w ostatniej minucie. Spróbuj za chwilę.",
				"Too many tickets were sent in the last minute. Try again shortly.",
			);
		const text = data?.errorNumber != null ? DESK[data.errorNumber] : undefined;
		if (text && data?.detail) {
			const captured = text.match?.exec(data.detail)?.[1] ?? "";
			// A captured catalog name (a field's label, a department) is Polish in the database: shown translated.
			if (i18n.lang === "en")
				return text.en
					? text.en.replace("$1", captured)
					: captured
						? data.detail.replace(captured, i18n.n(captured))
						: data.detail;
			return text.pl.replace("$1", i18n.n(captured) || captured);
		}
	}
	return problemMessage(error);
}
