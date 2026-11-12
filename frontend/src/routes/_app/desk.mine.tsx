import { createFileRoute } from "@tanstack/react-router";
import { ticketSearchSchema } from "#/features/desk/search";
import { TicketListPage } from "#/features/desk/TicketListPage";

export const Route = createFileRoute("/_app/desk/mine")({
	validateSearch: ticketSearchSchema,
	component: function Page() {
		const search = Route.useSearch();
		const navigate = Route.useNavigate();
		return (
			<TicketListPage
				scope="MINE"
				search={search}
				onSearch={(change) =>
					void navigate({ search: (s) => ({ ...s, ...change }), replace: true })
				}
			/>
		);
	},
});
