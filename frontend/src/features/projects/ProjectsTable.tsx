import { Table } from "@heroui/react";
import { useNavigate } from "@tanstack/react-router";
import type { ProjectResponse } from "#/api/models";
import { formatMoney } from "#/lib";
import { DueDate, StageChip } from "./StageChip";

/** Projects in due-date order; a row opens the project. `hide` drops columns that the page already shows. */
export function ProjectsTable({
	projects,
	hide = [],
	label = "Projects",
}: {
	projects: ProjectResponse[];
	hide?: ("client" | "team" | "owner")[];
	label?: string;
}) {
	const navigate = useNavigate();
	return (
		<Table className="rounded-none shadow-none">
			<Table.ScrollContainer>
				<Table.Content aria-label={label} className="min-w-[760px]">
					<Table.Header>
						<Table.Column isRowHeader>Project</Table.Column>
						{hide.includes("client") ? null : <Table.Column>Client</Table.Column>}
						{hide.includes("team") ? null : <Table.Column>Team</Table.Column>}
						{hide.includes("owner") ? null : <Table.Column>Owner</Table.Column>}
						<Table.Column>Stage</Table.Column>
						<Table.Column>Due</Table.Column>
						<Table.Column className="text-right">Budget</Table.Column>
					</Table.Header>
					<Table.Body>
						{projects.map((p) => (
							<Table.Row
								key={p.id}
								id={p.id}
								className="is-clickable"
								onAction={() =>
									void navigate({ to: "/projects/$projectId", params: { projectId: p.id } })
								}
							>
								<Table.Cell>
									<span className="font-medium text-foreground">{p.name}</span>
								</Table.Cell>
								{hide.includes("client") ? null : (
									<Table.Cell>
										<div className="flex flex-col">
											<span>{p.clientName}</span>
											<span className="text-xs text-muted">{p.clientCity ?? ""}</span>
										</div>
									</Table.Cell>
								)}
								{hide.includes("team") ? null : <Table.Cell>{p.teamName}</Table.Cell>}
								{hide.includes("owner") ? null : (
									<Table.Cell>
										<span className="font-mono text-xs text-muted">{p.ownerLogin}</span>
									</Table.Cell>
								)}
								<Table.Cell>
									<StageChip stage={p.stage} />
								</Table.Cell>
								<Table.Cell>
									<DueDate date={p.dueDate} overdue={p.overdue} />
								</Table.Cell>
								<Table.Cell className="text-right">
									<span className="figures text-xs">{formatMoney(p.budget)}</span>
								</Table.Cell>
							</Table.Row>
						))}
					</Table.Body>
				</Table.Content>
			</Table.ScrollContainer>
		</Table>
	);
}
