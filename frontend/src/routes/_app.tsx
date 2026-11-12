import { createFileRoute, redirect } from "@tanstack/react-router";
import { AppLayout } from "#/components/layout/AppLayout";
import { meQuery } from "#/features/auth/session";

// Every page behind the login. The user is loaded once here; pages read it with useMe().
export const Route = createFileRoute("/_app")({
	beforeLoad: async ({ context, location }) => {
		const me = await context.queryClient.query({ ...meQuery(), staleTime: "static" });
		if (!me) throw redirect({ to: "/login", search: { redirect: location.href } });
	},
	component: AppLayout,
});
