import { toast } from "@heroui/react";
import { forwardRef, useImperativeHandle, useState } from "react";
import { z } from "zod";
import type { UserResponse } from "#/api/models";
import { ApiError, FormDrawer } from "#/components/ui";
import { ROLES, roleLabel } from "#/features/types";
import { useAppForm } from "#/forms";
import { useChangeUserRole, useCreateUser, useResetUserPassword } from "./hooks";

export interface UserDrawerCommand {
	create: () => void;
	edit: (user: UserResponse) => void;
	resetPassword: (user: UserResponse) => void;
}

type Target =
	| { mode: "create" }
	| { mode: "role"; user: UserResponse }
	| { mode: "password"; user: UserResponse };

const roleOptions = ROLES.map((r) => ({ id: r, label: roleLabel[r] }));

export const UserDrawer = forwardRef<UserDrawerCommand>((_props, ref) => {
	const [target, setTarget] = useState<Target | null>(null);
	useImperativeHandle(
		ref,
		() => ({
			create: () => setTarget({ mode: "create" }),
			edit: (user) => setTarget({ mode: "role", user }),
			resetPassword: (user) => setTarget({ mode: "password", user }),
		}),
		[],
	);
	if (!target) return null;
	const close = () => setTarget(null);
	if (target.mode === "create") return <CreateUser onClose={close} />;
	if (target.mode === "role") return <ChangeRole user={target.user} onClose={close} />;
	return <ResetPassword user={target.user} onClose={close} />;
});
UserDrawer.displayName = "UserDrawer";

const createSchema = z.object({
	login: z
		.string()
		.trim()
		.regex(
			/^[a-z0-9._-]{3,64}$/,
			"3-64 characters: lowercase letters, digits, dot, underscore, hyphen",
		),
	displayName: z.string().trim().min(1, "Display name is required").max(200),
	role: z.enum(ROLES),
	password: z.string().min(8, "At least 8 characters"),
});

function CreateUser({ onClose }: { onClose: () => void }) {
	const mutation = useCreateUser((u) => {
		toast.success(`User ${u.login} created`);
		onClose();
	});
	const form = useAppForm({
		defaultValues: {
			login: "",
			displayName: "",
			role: "SALES" as (typeof ROLES)[number],
			password: "",
		},
		validators: { onChange: createSchema },
		onSubmit: ({ value }) => mutation.mutate(value),
	});
	return (
		<FormDrawer title="New user" onClose={onClose} onSubmit={() => void form.handleSubmit()}>
			<FormDrawer.Body>
				<form.AppField name="login">
					{(f) => <f.FormInput label="Login" placeholder="anna.nowak" autoFocus />}
				</form.AppField>
				<form.AppField name="displayName">
					{(f) => <f.FormInput label="Display name" placeholder="Anna Nowak" />}
				</form.AppField>
				<form.AppField name="role">
					{(f) => <f.FormSelect label="Role" options={roleOptions} />}
				</form.AppField>
				<form.AppField name="password">
					{(f) => (
						<f.FormInput
							label="Initial password"
							type="password"
							autoComplete="new-password"
							description="The user can change it after signing in."
						/>
					)}
				</form.AppField>
				<ApiError error={mutation.error} />
			</FormDrawer.Body>
			<FormDrawer.Footer submitLabel="Create user" isPending={mutation.isPending} />
		</FormDrawer>
	);
}

function ChangeRole({ user, onClose }: { user: UserResponse; onClose: () => void }) {
	const mutation = useChangeUserRole((u) => {
		toast.success(`${u.displayName} is now ${roleLabel[u.role as (typeof ROLES)[number]]}`);
		onClose();
	});
	const form = useAppForm({
		defaultValues: { role: user.role as (typeof ROLES)[number] },
		validators: { onChange: z.object({ role: z.enum(ROLES) }) },
		onSubmit: ({ value }) => mutation.mutate({ id: user.id, role: value.role }),
	});
	return (
		<FormDrawer
			title="Change role"
			description={`${user.displayName} · ${user.login}`}
			onClose={onClose}
			onSubmit={() => void form.handleSubmit()}
		>
			<FormDrawer.Body>
				<form.AppField name="role">
					{(f) => <f.FormSelect label="Role" options={roleOptions} />}
				</form.AppField>
				<ApiError error={mutation.error} />
			</FormDrawer.Body>
			<FormDrawer.Footer isPending={mutation.isPending} />
		</FormDrawer>
	);
}

function ResetPassword({ user, onClose }: { user: UserResponse; onClose: () => void }) {
	const mutation = useResetUserPassword(() => {
		toast.success(`Password of ${user.login} set; the account is unlocked`);
		onClose();
	});
	const form = useAppForm({
		defaultValues: { password: "" },
		validators: { onChange: z.object({ password: z.string().min(8, "At least 8 characters") }) },
		onSubmit: ({ value }) => mutation.mutate({ id: user.id, password: value.password }),
	});
	return (
		<FormDrawer
			title="Set password"
			description={`${user.displayName} · ${user.login}`}
			onClose={onClose}
			onSubmit={() => void form.handleSubmit()}
		>
			<FormDrawer.Body>
				<form.AppField name="password">
					{(f) => (
						<f.FormInput
							label="New password"
							type="password"
							autoComplete="new-password"
							autoFocus
						/>
					)}
				</form.AppField>
				<ApiError error={mutation.error} />
			</FormDrawer.Body>
			<FormDrawer.Footer submitLabel="Set password" isPending={mutation.isPending} />
		</FormDrawer>
	);
}
