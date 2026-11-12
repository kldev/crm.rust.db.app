import { useMutation } from "@tanstack/react-query";
import { forwardRef, useImperativeHandle, useState } from "react";
import { z } from "zod";
import { changeOwnPassword } from "#/api/endpoints/auth/auth";
import { ApiError, FormDrawer } from "#/components/ui";
import { useAppForm } from "#/forms";

export interface ChangePasswordCommand {
	open: () => void;
}

const schema = z
	.object({
		currentPassword: z.string().min(1, "Enter your current password"),
		newPassword: z.string().min(8, "At least 8 characters"),
		repeat: z.string(),
	})
	.refine((v) => v.newPassword === v.repeat, { message: "Passwords differ", path: ["repeat"] });

export const ChangePasswordDrawer = forwardRef<ChangePasswordCommand, { onSuccess: () => void }>(
	({ onSuccess }, ref) => {
		const [open, setOpen] = useState(false);
		useImperativeHandle(ref, () => ({ open: () => setOpen(true) }), []);
		if (!open) return null;
		return (
			<FormContent
				onClose={() => setOpen(false)}
				onSuccess={() => {
					setOpen(false);
					onSuccess();
				}}
			/>
		);
	},
);
ChangePasswordDrawer.displayName = "ChangePasswordDrawer";

function FormContent({ onClose, onSuccess }: { onClose: () => void; onSuccess: () => void }) {
	const mutation = useMutation({
		mutationFn: (v: z.infer<typeof schema>) =>
			changeOwnPassword({ currentPassword: v.currentPassword, newPassword: v.newPassword }),
		onSuccess,
	});
	const form = useAppForm({
		defaultValues: { currentPassword: "", newPassword: "", repeat: "" },
		validators: { onChange: schema },
		onSubmit: ({ value }) => mutation.mutate(value),
	});

	return (
		<FormDrawer title="Change password" onClose={onClose} onSubmit={() => void form.handleSubmit()}>
			<FormDrawer.Body>
				<form.AppField name="currentPassword">
					{(f) => (
						<f.FormInput
							label="Current password"
							type="password"
							autoComplete="current-password"
							autoFocus
						/>
					)}
				</form.AppField>
				<form.AppField name="newPassword">
					{(f) => (
						<f.FormInput
							label="New password"
							type="password"
							autoComplete="new-password"
							description="At least 8 characters."
						/>
					)}
				</form.AppField>
				<form.AppField name="repeat">
					{(f) => (
						<f.FormInput label="Repeat new password" type="password" autoComplete="new-password" />
					)}
				</form.AppField>
				<ApiError error={mutation.error} />
			</FormDrawer.Body>
			<FormDrawer.Footer submitLabel="Change password" isPending={mutation.isPending} />
		</FormDrawer>
	);
}
