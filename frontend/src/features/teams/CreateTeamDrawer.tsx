import { forwardRef, useImperativeHandle, useState } from "react";
import { z } from "zod";
import type { TeamResponse } from "#/api/models";
import { ApiError, FormDrawer } from "#/components/ui";
import { useManagers } from "#/features/users/hooks";
import { useAppForm } from "#/forms";
import { useCreateTeam } from "./hooks";

export interface CreateTeamCommand {
	open: () => void;
}

const schema = z.object({
	name: z.string().trim().min(1, "Name is required").max(200),
	leadUserId: z.string().min(1, "Choose the lead"),
});

export const CreateTeamDrawer = forwardRef<
	CreateTeamCommand,
	{ onCreated: (t: TeamResponse) => void }
>(({ onCreated }, ref) => {
	const [open, setOpen] = useState(false);
	useImperativeHandle(ref, () => ({ open: () => setOpen(true) }), []);
	if (!open) return null;
	return (
		<FormContent
			onClose={() => setOpen(false)}
			onCreated={(t) => {
				setOpen(false);
				onCreated(t);
			}}
		/>
	);
});
CreateTeamDrawer.displayName = "CreateTeamDrawer";

function FormContent({
	onClose,
	onCreated,
}: {
	onClose: () => void;
	onCreated: (t: TeamResponse) => void;
}) {
	const managers = useManagers(true);
	const mutation = useCreateTeam(onCreated);
	const form = useAppForm({
		defaultValues: { name: "", leadUserId: "" },
		validators: { onChange: schema },
		onSubmit: ({ value }) =>
			mutation.mutate({ name: value.name.trim(), leadUserId: Number(value.leadUserId) }),
	});
	return (
		<FormDrawer
			title="New team"
			description="Led by an active manager."
			onClose={onClose}
			onSubmit={() => void form.handleSubmit()}
		>
			<FormDrawer.Body>
				<form.AppField name="name">
					{(f) => <f.FormInput label="Team name" placeholder="North" autoFocus />}
				</form.AppField>
				<form.AppField name="leadUserId">
					{(f) => (
						<f.FormSelect
							label="Lead"
							placeholder={managers.isPending ? "Loading managers…" : "Choose a manager"}
							options={(managers.data ?? []).map((m) => ({
								id: String(m.id),
								label: m.displayName,
								description: m.login,
							}))}
						/>
					)}
				</form.AppField>
				<ApiError error={mutation.error} />
			</FormDrawer.Body>
			<FormDrawer.Footer submitLabel="Create team" isPending={mutation.isPending} />
		</FormDrawer>
	);
}
