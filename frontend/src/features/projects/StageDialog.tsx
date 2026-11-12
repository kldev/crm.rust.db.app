import { Button, Label, Modal, TextArea, TextField } from "@heroui/react";
import { useState } from "react";
import type { ProjectResponse } from "#/api/models";
import { ApiError } from "#/components/ui";
import { type Stage, stageAction, stageLabel } from "#/features/types";
import { useChangeProjectStage } from "./hooks";

/** Confirms a stage move with an optional comment; both are written in one transaction by the procedure. */
export function StageDialog({
	project,
	target,
	onClose,
	onChanged,
}: {
	project: ProjectResponse;
	target: Stage | null;
	onClose: () => void;
	onChanged: (p: ProjectResponse) => void;
}) {
	const [comment, setComment] = useState("");
	const mutation = useChangeProjectStage(project.id, (p) => {
		setComment("");
		onChanged(p);
	});
	const close = () => {
		setComment("");
		mutation.reset();
		onClose();
	};

	return (
		<Modal>
			<Modal.Backdrop isOpen={target !== null} onOpenChange={(open) => !open && close()}>
				<Modal.Container size="md">
					<Modal.Dialog>
						<Modal.CloseTrigger />
						<Modal.Header>
							<Modal.Heading className="font-display text-2xl">
								{target ? `${stageAction[target]} project` : ""}
							</Modal.Heading>
						</Modal.Header>
						<Modal.Body>
							<div className="flex flex-col gap-4">
								<p className="text-sm text-muted">
									{project.name}: {stageLabel[project.stage as Stage]} →{" "}
									<strong className="text-foreground">{target ? stageLabel[target] : ""}</strong>
								</p>
								<TextField value={comment} onChange={setComment} fullWidth>
									<Label>Comment (optional)</Label>
									<TextArea rows={3} placeholder="Why the project moves on, for the team" />
								</TextField>
								<ApiError error={mutation.error} />
							</div>
						</Modal.Body>
						<Modal.Footer>
							<Button slot="close" variant="secondary">
								Cancel
							</Button>
							<Button
								variant={target === "DECLINED" ? "danger" : undefined}
								isPending={mutation.isPending}
								onPress={() =>
									target &&
									mutation.mutate({
										stage: target,
										comment: comment.trim() || null,
										rowVersion: project.rowVersion,
									})
								}
							>
								{target ? stageAction[target] : "Confirm"}
							</Button>
						</Modal.Footer>
					</Modal.Dialog>
				</Modal.Container>
			</Modal.Backdrop>
		</Modal>
	);
}
