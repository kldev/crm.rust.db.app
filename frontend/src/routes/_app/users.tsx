import { createFileRoute, redirect } from "@tanstack/react-router";
import { z } from "zod";
import { can, meQuery, type Role } from "#/features/auth/session";
import { ROLES } from "#/features/types";
import { UsersPage } from "#/features/users/UsersPage";

export const Route = createFileRoute("/_app/users")({
	validateSearch: z.object({ role: z.enum(ROLES).optional() }),
	// SALES users get 403 from /api/users; do not show them an empty page.
	beforeLoad: async ({ context }) => {
		const me = await context.queryClient.query({ ...meQuery(), staleTime: "static" });
		if (!me || !can.seeUsers(me.role as Role)) throw redirect({ to: "/" });
	},
	component: UsersPage,
});
