import { Card, Table } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Contact } from "lucide-react";
import { PageHeader } from "#/components/layout/AppLayout";
import { ApiError, EmptyState, TableSkeleton } from "#/components/ui";
import { useI18n } from "#/i18n";
import { peopleQuery } from "./hooks";

/** People who report tickets and do the work named by agents; read-only. */
export function PeoplePage() {
	const { t, n } = useI18n();
	const query = useQuery(peopleQuery());
	return (
		<>
			<PageHeader
				eyebrow={t("Service desk", "Service desk")}
				title={t("Osoby", "People")}
				description={t(
					"Zgłaszający i osoby realizujące; nie logują się do systemu.",
					"Requesters and the people doing the work; they do not sign in.",
				)}
			/>
			<Card className="overflow-hidden p-0">
				{query.isPending ? (
					<TableSkeleton />
				) : query.isError ? (
					<div className="p-4">
						<ApiError error={query.error} />
					</div>
				) : query.data.length === 0 ? (
					<EmptyState icon={Contact} title={t("Brak osób", "No people")} />
				) : (
					<Table className="rounded-none shadow-none">
						<Table.ScrollContainer>
							<Table.Content aria-label={t("Osoby", "People")} className="min-w-[640px]">
								<Table.Header>
									<Table.Column isRowHeader>{t("Imię i nazwisko", "Name")}</Table.Column>
									<Table.Column>{t("Dział", "Department")}</Table.Column>
									<Table.Column>{t("E-mail", "E-mail")}</Table.Column>
									<Table.Column>{t("Telefon", "Phone")}</Table.Column>
								</Table.Header>
								<Table.Body>
									{query.data.map((p) => (
										<Table.Row key={p.id} id={p.id}>
											<Table.Cell>
												{p.firstName} {p.lastName}
											</Table.Cell>
											<Table.Cell>{n(p.department)}</Table.Cell>
											<Table.Cell>
												<a className="text-sm hover:underline" href={`mailto:${p.email}`}>
													{p.email}
												</a>
											</Table.Cell>
											<Table.Cell>
												<span className="figures text-xs">{p.phone ?? "—"}</span>
											</Table.Cell>
										</Table.Row>
									))}
								</Table.Body>
							</Table.Content>
						</Table.ScrollContainer>
					</Table>
				)}
			</Card>
		</>
	);
}
