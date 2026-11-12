import { AlertDialog, Button } from "@heroui/react";
import type { ReactNode } from "react";
import { ApiError } from "./ApiError";

export function ConfirmDialog({
	isOpen,
	title,
	description,
	confirmLabel,
	isPending,
	error,
	onConfirm,
	onClose,
}: {
	isOpen: boolean;
	title: string;
	description: ReactNode;
	confirmLabel: string;
	isPending?: boolean;
	error?: unknown;
	onConfirm: () => void;
	onClose: () => void;
}) {
	return (
		<AlertDialog>
			<AlertDialog.Backdrop isOpen={isOpen} onOpenChange={(open) => !open && onClose()}>
				<AlertDialog.Container size="sm">
					<AlertDialog.Dialog>
						<AlertDialog.Header>
							<AlertDialog.Icon status="danger" />
							<AlertDialog.Heading>{title}</AlertDialog.Heading>
						</AlertDialog.Header>
						<AlertDialog.Body>
							<div className="flex flex-col gap-3">
								<div className="text-sm text-muted">{description}</div>
								<ApiError error={error} />
							</div>
						</AlertDialog.Body>
						<AlertDialog.Footer>
							<Button slot="close" variant="secondary">
								Cancel
							</Button>
							<Button variant="danger" isPending={isPending} onPress={onConfirm}>
								{confirmLabel}
							</Button>
						</AlertDialog.Footer>
					</AlertDialog.Dialog>
				</AlertDialog.Container>
			</AlertDialog.Backdrop>
		</AlertDialog>
	);
}
