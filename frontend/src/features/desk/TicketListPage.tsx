import { Button, Card, SearchField, ToggleButton, ToggleButtonGroup } from "@heroui/react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { Inbox, Plus } from "lucide-react";
import { useRef, useState } from "react";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, EmptyState, LoadMore, SelectFilter, TableSkeleton } from "#/components/ui";
import { useDebouncedValue } from "#/hooks/useDebouncedValue";
import { useNow } from "#/hooks/useNow";
import { useOnDemandTotal } from "#/hooks/useOnDemandTotal";
import { useI18n } from "#/i18n";
import { TicketDialogs, type TicketDialogsCommand } from "./components/TicketDialogs";
import { TicketsTable } from "./components/TicketsTable";
import { WorkActions } from "./components/WorkActions";
import { deskCatalogQuery, ticketsCountQuery, ticketsQuery } from "./hooks";
import type { TicketSearch } from "./search";
import { DEADLINE_FILTERS, PRIORITIES, priorityLabel, type Scope } from "./types";

/**
 * Pool, my tickets and all tickets: one page, three scopes. Filters live in the URL; a filter change starts the
 * list from the first page (it is a new query key). The pool and the lists refresh every 30 s on their own.
 */
export function TicketListPage({
	scope,
	search,
	onSearch,
}: {
	scope: Scope;
	search: TicketSearch;
	onSearch: (change: Partial<TicketSearch>) => void;
}) {
	const i18n = useI18n();
	const { t, n } = i18n;
	const now = useNow();
	const dialogs = useRef<TicketDialogsCommand>(null);
	const catalog = useQuery(deskCatalogQuery());

	const [text, setText] = useState(search.q ?? "");
	const q = useDebouncedValue(text.trim(), 300);
	if ((search.q ?? "") !== q) onSearch({ q: q || undefined });

	const filters = {
		scope,
		search: search.q,
		categoryKey: search.category,
		subcategoryKey: search.sub,
		priority: search.priority,
		deadline: search.deadline,
		completion: scope === "ALL" ? search.completion : undefined,
	};
	const query = useInfiniteQuery(ticketsQuery(filters));
	const total = useOnDemandTotal(ticketsCountQuery(filters));
	const tickets = query.data?.pages.flatMap((p) => p.items) ?? [];

	const category = catalog.data?.categories.find((c) => c.key === search.category);
	const header = {
		POOL: {
			title: t("Pula", "Pool"),
			description: t(
				"Zgłoszenia czekające na agenta, najpilniejsze na górze.",
				"Tickets waiting for an agent, the most urgent on top.",
			),
		},
		MINE: {
			title: t("Moje zgłoszenia", "My tickets"),
			description: t("Zgłoszenia, które teraz realizujesz.", "Tickets you are working on now."),
		},
		ALL: {
			title: t("Wszystkie zgłoszenia", "All tickets"),
			description: t(
				"Otwarte według terminu, potem zrealizowane, najnowsze najpierw.",
				"Open ones by deadline, then resolved ones, newest first.",
			),
		},
	}[scope];

	return (
		<>
			<PageHeader
				eyebrow={t("Service desk", "Service desk")}
				title={header.title}
				description={header.description}
				actions={
					<Link to="/desk/new">
						<Button>
							<Plus className="size-4" />
							{t("Nowe zgłoszenie", "New ticket")}
						</Button>
					</Link>
				}
			/>

			<Card className="overflow-hidden p-0">
				<div className="flex flex-col flex-wrap gap-3 border-b border-separator p-4 sm:flex-row sm:items-center">
					<SearchField
						aria-label={t("Szukaj zgłoszeń", "Search tickets")}
						className="w-full sm:max-w-xs"
						value={text}
						onChange={setText}
					>
						<SearchField.Group>
							<SearchField.SearchIcon />
							<SearchField.Input
								placeholder={t("Tytuł albo numer HAL-…", "Title or number HAL-…")}
							/>
							<SearchField.ClearButton />
						</SearchField.Group>
					</SearchField>
					<SelectFilter
						label={t("Kategoria", "Category")}
						className="w-full sm:w-56"
						value={search.category}
						options={(catalog.data?.categories ?? []).map((c) => ({ id: c.key, label: n(c.name) }))}
						onChange={(value) => onSearch({ category: value, sub: undefined })}
					/>
					{category ? (
						<SelectFilter
							label={t("Podkategoria", "Subcategory")}
							className="w-full sm:w-64"
							value={search.sub}
							options={category.subcategories.map((s) => ({
								id: s.key,
								label: s.market ? `${n(s.market)} · ${n(s.name)}` : n(s.name),
							}))}
							onChange={(value) => onSearch({ sub: value })}
						/>
					) : null}
					<SelectFilter
						label={t("Priorytet", "Priority")}
						value={search.priority}
						options={PRIORITIES.map((p) => ({ id: p, label: priorityLabel(p, i18n) }))}
						onChange={(value) => onSearch({ priority: value as TicketSearch["priority"] })}
					/>
					<SelectFilter
						label={t("Termin", "Deadline")}
						value={search.deadline}
						options={DEADLINE_FILTERS.map((d) => ({
							id: d,
							label:
								d === "OVERDUE"
									? t("Po terminie", "Overdue")
									: t("Kończy się (< 1 h)", "Due soon (< 1 h)"),
						}))}
						onChange={(value) => onSearch({ deadline: value as TicketSearch["deadline"] })}
					/>
					{scope === "ALL" ? (
						<ToggleButtonGroup
							aria-label={t("Realizacja", "Completion")}
							selectionMode="single"
							size="sm"
							selectedKeys={new Set([search.completion ?? "ANY"])}
							onSelectionChange={(keys) => {
								const key = [...keys][0];
								onSearch({ completion: key === "OPEN" || key === "DONE" ? key : undefined });
							}}
						>
							<ToggleButton id="ANY">{t("Wszystkie", "All")}</ToggleButton>
							<ToggleButton id="OPEN">{t("Otwarte", "Open")}</ToggleButton>
							<ToggleButton id="DONE">{t("Zrealizowane", "Resolved")}</ToggleButton>
						</ToggleButtonGroup>
					) : null}
				</div>

				{query.isPending ? (
					<TableSkeleton />
				) : query.isError ? (
					<div className="p-4">
						<ApiError
							error={query.error}
							title={t("Nie udało się wczytać zgłoszeń", "Tickets could not be loaded")}
						/>
					</div>
				) : tickets.length === 0 ? (
					<EmptyState
						icon={Inbox}
						title={
							scope === "POOL"
								? t("Pula jest pusta", "The pool is empty")
								: t("Brak zgłoszeń", "No tickets")
						}
						description={t("Zmień albo wyczyść filtry.", "Change or clear the filters.")}
					/>
				) : (
					<>
						<TicketsTable
							tickets={tickets}
							catalog={catalog.data}
							now={now}
							actions={(ticket) => <WorkActions ticket={ticket} dialogs={dialogs} />}
						/>
						<LoadMore
							noun={t("zgłoszeń", "tickets")}
							shown={tickets.length}
							hasMore={query.hasNextPage}
							isLoading={query.isFetchingNextPage}
							onLoadMore={() => void query.fetchNextPage()}
							total={total}
						/>
					</>
				)}
			</Card>

			<TicketDialogs ref={dialogs} />
		</>
	);
}
