import { Button, Drawer } from "@heroui/react";
import { createContext, type ReactNode, useContext, useId } from "react";

const FormIdContext = createContext<string | null>(null);

/**
 * A form in a right-hand drawer: fields in the body, the submit button in the footer, joined by the form id,
 * so Enter in a field submits too. The drawer is open while it is mounted; closing calls onClose.
 */
export function FormDrawer({
	title,
	description,
	onClose,
	onSubmit,
	children,
}: {
	title: string;
	description?: string;
	onClose: () => void;
	onSubmit: () => void;
	children: ReactNode;
}) {
	const formId = useId();
	return (
		<FormIdContext.Provider value={formId}>
			<Drawer>
				<Drawer.Backdrop isOpen onOpenChange={(open) => !open && onClose()}>
					<Drawer.Content placement="right" className="w-full sm:max-w-md">
						<Drawer.Dialog>
							<Drawer.CloseTrigger />
							<Drawer.Header>
								<div className="flex flex-col gap-1">
									<Drawer.Heading className="font-display text-2xl font-semibold">
										{title}
									</Drawer.Heading>
									{description ? <p className="text-sm text-muted">{description}</p> : null}
								</div>
							</Drawer.Header>
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
						</Drawer.Dialog>
					</Drawer.Content>
				</Drawer.Backdrop>
			</Drawer>
		</FormIdContext.Provider>
	);
}

FormDrawer.Body = function FormDrawerBody({ children }: { children: ReactNode }) {
	return (
		<Drawer.Body>
			<div className="flex flex-col gap-5 pb-2">{children}</div>
		</Drawer.Body>
	);
};

FormDrawer.Footer = function FormDrawerFooter({
	submitLabel = "Save",
	isPending,
}: {
	submitLabel?: string;
	isPending?: boolean;
}) {
	const formId = useContext(FormIdContext) ?? undefined;
	return (
		<Drawer.Footer>
			<Button slot="close" variant="secondary">
				Cancel
			</Button>
			<Button type="submit" form={formId} isPending={isPending}>
				{submitLabel}
			</Button>
		</Drawer.Footer>
	);
};
