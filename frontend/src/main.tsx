import "#/styles/global.css";

import { Toast } from "@heroui/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createRouter, RouterProvider } from "@tanstack/react-router";
import axios from "axios";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { LanguageProvider } from "#/i18n";
import { routeTree } from "#/routeTree.gen";

const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			staleTime: 30_000,
			refetchOnWindowFocus: false,
			// A 4xx will not get better on retry (validation, permission, not found, session).
			retry: (count, error) =>
				!(axios.isAxiosError(error) && (error.response?.status ?? 500) < 500) && count < 2,
		},
	},
});

const router = createRouter({
	routeTree,
	context: { queryClient },
	defaultPreload: "intent",
	defaultPreloadStaleTime: 0,
	scrollRestoration: true,
});

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
	interface StaticDataRouteOption {
		/** The page uses the whole screen width (wide tables, charts) instead of the readable column. */
		wide?: boolean;
	}
}

const root = document.getElementById("root");
if (!root) throw new Error("#root is missing in index.html");

createRoot(root).render(
	<StrictMode>
		<LanguageProvider>
			<QueryClientProvider client={queryClient}>
				<RouterProvider router={router} />
				<Toast.Provider placement="bottom end" />
			</QueryClientProvider>
		</LanguageProvider>
	</StrictMode>,
);
