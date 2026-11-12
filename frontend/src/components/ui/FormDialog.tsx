import { Button, Modal } from "@heroui/react";
import { createContext, type ReactNode, useContext, useId } from "react";

const FormIdContext = createContext<string | null>(null);

/** wide: a two-column create / edit form of a record; compact: a short one-column form (passwords, a role). */
type FormDialogSize = "wide" | "compact";

const WIDTH: Record<FormDialogSize, string> = {
	wide: "sm:max-w-3xl",
	compact: "sm:max-w-lg",
};

/**
 * A create / edit form in a dialog in the middle of the screen: fields in the body, the submit button in the footer,
 * joined by the form id, so Enter in a field submits too. The dialog is open while it is mounted; closing calls
 * onClose. The body scrolls inside the dialog, so a long form keeps its header and buttons on screen.
 */
export function FormDialog({
	title,
	description,
	size = "wide",
	onClose,
	onSubmit,
	children,
}: {
	title: string;
	description?: string;
	size?: FormDialogSize;
	onClose: () => void;
	onSubmit: () => void;
	children: ReactNode;
}) {
	const formId = useId();
	return (
		<FormIdContext.Provider value={formId}>
			<Modal>
				<Modal.Backdrop isOpen onOpenChange={(open) => !open && onClose()}>
					<Modal.Container placement="center" scroll="inside">
						<Modal.Dialog className={`w-full ${WIDTH[size]}`}>
							<Modal.CloseTrigger />
							<Modal.Header className="border-b border-separator pb-4">
								<div className="flex flex-col gap-1">
									<Modal.Heading className="font-display text-xl font-semibold">
										{title}
									</Modal.Heading>
									{description ? <p className="text-sm text-muted">{description}</p> : null}
								</div>
							</Modal.Header>
							<form
								id={formId}
								noValidate
								className="contents"
								onSubmit={(event) => {
									event.preventDefault();
									onSubmit();
								}}
							>
								{children}
							</form>
						</Modal.Dialog>
					</Modal.Container>
				</Modal.Backdrop>
			</Modal>
		</FormIdContext.Provider>
	);
}

/** Fields: one column on a phone, two from sm up (columns=2). FormDialog.Full spans both columns. */
FormDialog.Body = function FormDialogBody({
	columns = 2,
	children,
}: {
	columns?: 1 | 2;
	children: ReactNode;
}) {
	return (
		<Modal.Body>
			<div className={`grid gap-5 py-2 ${columns === 2 ? "sm:grid-cols-2" : "grid-cols-1"}`}>
				{children}
			</div>
		</Modal.Body>
	);
};

/** A full-width row of a two-column body; an empty one (no API error yet) takes no space. */
FormDialog.Full = function FormDialogFull({ children }: { children: ReactNode }) {
	return <div className="empty:hidden sm:col-span-2">{children}</div>;
};

FormDialog.Footer = function FormDialogFooter({
	submitLabel = "Save",
	isPending,
}: {
	submitLabel?: string;
	isPending?: boolean;
}) {
	const formId = useContext(FormIdContext) ?? undefined;
	return (
		<Modal.Footer className="border-t border-separator pt-4">
			<Button slot="close" variant="secondary">
				Cancel
			</Button>
			<Button type="submit" form={formId} isPending={isPending}>
				{submitLabel}
			</Button>
		</Modal.Footer>
	);
};
