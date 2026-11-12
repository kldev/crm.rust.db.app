import { forwardRef, useImperativeHandle, useState } from "react";
import { z } from "zod";
import type { ClientResponse } from "#/api/models";
import { ApiError, FormDrawer } from "#/components/ui";
import { can, useMe } from "#/features/auth/session";
import { CITIES, CLIENT_STATUSES, clientStatusLabel } from "#/features/types";
import { useSalesUsers } from "#/features/users/hooks";
import { useAppForm } from "#/forms";
import { useCreateClient, useUpdateClient } from "./hooks";

export interface ClientFormCommand {
	create: () => void;
	edit: (client: ClientResponse) => void;
}

const schema = z.object({
	name: z.string().trim().min(1, "Name is required").max(200),
	nip: z
		.string()
		.trim()
		.refine((v) => v === "" || /^\d{10}$/.test(v.replace(/[\s-]/g, "")), "NIP has 10 digits"),
	email: z
		.string()
		.trim()
		.refine((v) => v === "" || z.email().safeParse(v).success, "Not a valid e-mail"),
	phone: z.string().trim().max(30),
	city: z.string().trim().max(100),
	status: z.enum(CLIENT_STATUSES),
	assignedUserId: z.string(),
	note: z.string().max(2000),
});
type Values = z.infer<typeof schema>;

const orNull = (v: string) => (v.trim() === "" ? null : v.trim());

type Target = { mode: "create" } | { mode: "edit"; client: ClientResponse };

export const ClientFormDrawer = forwardRef<
	ClientFormCommand,
	{ onSaved: (client: ClientResponse, created: boolean) => void }
>(({ onSaved }, ref) => {
	const [target, setTarget] = useState<Target | null>(null);
	useImperativeHandle(
		ref,
		() => ({
			create: () => setTarget({ mode: "create" }),
			edit: (client) => setTarget({ mode: "edit", client }),
		}),
		[],
	);
	if (!target) return null;
	return (
		<FormContent
			target={target}
			onClose={() => setTarget(null)}
			onSaved={(client) => {
				setTarget(null);
				onSaved(client, target.mode === "create");
			}}
		/>
	);
});
ClientFormDrawer.displayName = "ClientFormDrawer";

function FormContent({
	target,
	onClose,
	onSaved,
}: {
	target: Target;
	onClose: () => void;
	onSaved: (client: ClientResponse) => void;
}) {
	const me = useMe();
	const editing = target.mode === "edit" ? target.client : null;
	const create = useCreateClient(onSaved);
	const update = useUpdateClient(editing?.id ?? 0, onSaved);
	const mutation = editing ? update : create;
	const salesUsers = useSalesUsers(can.assignClients(me.role));

	const form = useAppForm({
		defaultValues: {
			name: editing?.name ?? "",
			nip: editing?.nip ?? "",
			email: editing?.email ?? "",
			phone: editing?.phone ?? "",
			city: editing?.city ?? "",
			status: (editing?.status ?? "LEAD") as Values["status"],
			assignedUserId: editing?.assignedUserId ? String(editing.assignedUserId) : "",
			note: editing?.note ?? "",
		} satisfies Values,
		validators: { onChange: schema },
		onSubmit: ({ value }) =>
			mutation.mutate({
				name: value.name.trim(),
				nip: orNull(value.nip.replace(/[\s-]/g, "")),
				email: orNull(value.email),
				phone: orNull(value.phone),
				city: orNull(value.city),
				status: value.status,
				// SALES: the procedure assigns the client to the actor.
				assignedUserId: value.assignedUserId
					? Number(value.assignedUserId)
					: (editing?.assignedUserId ?? null),
				note: orNull(value.note),
				rowVersion: editing?.rowVersion ?? null,
			}),
	});

	return (
		<FormDrawer
			title={editing ? "Edit client" : "New client"}
			description={editing ? editing.name : "A lead, an active client or one to archive."}
			onClose={onClose}
			onSubmit={() => void form.handleSubmit()}
		>
			<FormDrawer.Body>
				<form.AppField name="name">
					{(f) => (
						<f.FormInput label="Company name" placeholder="Zielony Most Sp. z o.o." autoFocus />
					)}
				</form.AppField>
				<div className="grid grid-cols-2 gap-4">
					<form.AppField name="nip">
						{(f) => <f.FormInput label="NIP" placeholder="1234567890" inputMode="numeric" />}
					</form.AppField>
					<form.AppField name="status">
						{(f) => (
							<f.FormSelect
								label="Status"
								options={CLIENT_STATUSES.map((s) => ({ id: s, label: clientStatusLabel[s] }))}
							/>
						)}
					</form.AppField>
				</div>
				<form.AppField name="email">
					{(f) => <f.FormInput label="E-mail" type="email" placeholder="office@example.com" />}
				</form.AppField>
				<div className="grid grid-cols-2 gap-4">
					<form.AppField name="phone">
						{(f) => <f.FormInput label="Phone" type="tel" placeholder="+48 500 000 000" />}
					</form.AppField>
					<form.AppField name="city">
						{(f) => (
							<f.FormSelect label="City" options={CITIES.map((c) => ({ id: c, label: c }))} />
						)}
					</form.AppField>
				</div>
				{can.assignClients(me.role) ? (
					<form.AppField name="assignedUserId">
						{(f) => (
							<f.FormSelect
								label="Account owner"
								placeholder={salesUsers.isPending ? "Loading sales users…" : "Unassigned"}
								options={(salesUsers.data ?? []).map((u) => ({
									id: String(u.id),
									label: u.displayName,
									description: u.login,
								}))}
								description="A client is owned by one active sales user."
							/>
						)}
					</form.AppField>
				) : null}
				<form.AppField name="note">
					{(f) => <f.FormTextArea label="Note" placeholder="Anything worth remembering" rows={3} />}
				</form.AppField>
				<ApiError error={mutation.error} />
			</FormDrawer.Body>
			<FormDrawer.Footer
				submitLabel={editing ? "Save changes" : "Create client"}
				isPending={mutation.isPending}
			/>
		</FormDrawer>
	);
}
