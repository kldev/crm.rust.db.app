import axios, { type AxiosError, type AxiosRequestConfig } from "axios";

/**
 * One axios instance for the generated client. Same origin as the API (Spring serves the app, Vite proxies in
 * development), so the session cookie travels on its own. Spring's CSRF cookie and header have axios' default
 * names (XSRF-TOKEN / X-XSRF-TOKEN), so every write carries the token without extra code.
 */
export const http = axios.create({
	withCredentials: true,
	xsrfCookieName: "XSRF-TOKEN",
	xsrfHeaderName: "X-XSRF-TOKEN",
});

/** RFC 9457 body of every API error; `errorNumber` is the THROW number of the procedure. */
export interface Problem {
	title?: string;
	detail?: string;
	status?: number;
	errorNumber?: number | null;
}

/** An expired session sends the user to the login page; the auth endpoints handle 401 themselves. */
http.interceptors.response.use(undefined, (error: AxiosError) => {
	const url = error.config?.url ?? "";
	if (error.response?.status === 401 && !url.startsWith("/api/auth/")) {
		const here = window.location.pathname + window.location.search;
		if (!window.location.pathname.startsWith("/login")) {
			window.location.assign(`/login?redirect=${encodeURIComponent(here)}`);
		}
	}
	return Promise.reject(error);
});

export const customInstance = <T>(config: AxiosRequestConfig, options?: AxiosRequestConfig) =>
	http.request<T>({ ...config, ...options }).then((response) => response.data);

export type ErrorType<E> = AxiosError<E>;
export type BodyType<B> = B;

/** A human message from any error of the API (ProblemDetail detail, field errors, or a network failure). */
export function problemMessage(error: unknown): string {
	if (axios.isAxiosError<Problem & { errors?: { defaultMessage?: string }[] }>(error)) {
		const data = error.response?.data;
		if (data?.detail) return data.detail;
		if (!error.response)
			return "The server cannot be reached. Check your connection and try again.";
		return `Request failed (${error.response.status}).`;
	}
	return error instanceof Error ? error.message : "Something went wrong.";
}

/** The THROW number of the procedure behind an error, when the API exposes it. */
export function problemNumber(error: unknown): number | undefined {
	if (axios.isAxiosError<Problem>(error)) return error.response?.data?.errorNumber ?? undefined;
	return undefined;
}
