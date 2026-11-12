import { Card, SearchField, Table } from "@heroui/react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { UsersRound } from "lucide-react";
import { useState } from "react";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, EmptyState, LoadMore, TableSkeleton, UserAvatar } from "#/components/ui";
import { useDebouncedValue } from "#/hooks/useDebouncedValue";
import { useI18n } from "#/i18n";
import { agentsQuery } from "./hooks";

/** Agents - every active CRM user - with the number of tickets each one has in progress. */
export function AgentsPage() {
	const { t } = useI18n();
	const [text, setText] = useState("");
	const search = useDebouncedValue(text.trim(), 300);
	const query = useInfiniteQuery(agentsQuery(search || undefined));
	const agents = query.data?.pages.flatMap((p) => p.items) ?? [];
	return (
		<>
			<PageHeader
				eyebrow={t("Service desk", "Service desk")}
				title={t("Agenci", "Agents")}
				description={t(
					"Każdy aktywny użytkownik CRM może realizować zgłoszenia.",
					"Every active CRM user can work on tickets.",
				)}
			/>
			<Card className="overflow-hidden p-0">
				<div className="border-b border-separator p-4">
					<SearchField
						aria-label={t("Szukaj agenta", "Search agents")}
						className="w-full sm:max-w-xs"
						value={text}
						onChange={setText}
					>
						<SearchField.Group>
							<SearchField.SearchIcon />
							<SearchField.Input
								placeholder={t("Początek imienia albo loginu…", "Start of a name or login…")}
							/>
							<SearchField.ClearButton />
						</SearchField.Group>
					</SearchField>
				</div>
				{query.isPending ? (
					<TableSkeleton />
				) : query.isError ? (
					<div className="p-4">
						<ApiError error={query.error} />
					</div>
				) : agents.length === 0 ? (
					<EmptyState icon={UsersRound} title={t("Brak agentów", "No agents")} />
				) : (
					<>
						<Table className="rounded-none shadow-none">
							<Table.ScrollContainer>
								<Table.Content aria-label={t("Agenci", "Agents")}>
									<Table.Header>
										<Table.Column isRowHeader>{t("Agent", "Agent")}</Table.Column>
										<Table.Column>{t("Login", "Login")}</Table.Column>
										<Table.Column className="text-right">
											{t("W realizacji", "In progress")}
										</Table.Column>
									</Table.Header>
									<Table.Body>
										{agents.map((a) => (
											<Table.Row key={a.id} id={a.id}>
												<Table.Cell>
													<span className="flex items-center gap-3">
														<UserAvatar name={a.displayName} />
														{a.displayName}
													</span>
												</Table.Cell>
												<Table.Cell>
													<span className="font-mono text-xs text-muted">{a.login}</span>
												</Table.Cell>
												<Table.Cell className="figures text-right">{a.inProgress}</Table.Cell>
											</Table.Row>
										))}
									</Table.Body>
								</Table.Content>
							</Table.ScrollContainer>
						</Table>
						<LoadMore
							noun={t("agentów", "agents")}
							shown={agents.length}
							hasMore={query.hasNextPage}
							isLoading={query.isFetchingNextPage}
							onLoadMore={() => void query.fetchNextPage()}
						/>
					</>
				)}
			</Card>
		</>
	);
}
