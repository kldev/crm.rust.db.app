import { createFileRoute, redirect } from "@tanstack/react-router";
import { z } from "zod";
import { LoginPage } from "#/features/auth/LoginPage";
import { meQuery } from "#/features/auth/session";

export const Route = createFileRoute("/login")({
	validateSearch: z.object({ redirect: z.string().optional() }),
	beforeLoad: async ({ context, search }) => {
		const me = await context.queryClient.query({ ...meQuery(), staleTime: "static" });
		if (me) throw redirect({ to: search.redirect ?? "/" });
	},
	component: LoginPage,
});
