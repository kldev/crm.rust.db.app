import { createFileRoute, redirect } from "@tanstack/react-router";

// The desk starts at the pool.
export const Route = createFileRoute("/_app/desk/")({
	beforeLoad: () => {
		throw redirect({ to: "/desk/pool" });
	},
});
