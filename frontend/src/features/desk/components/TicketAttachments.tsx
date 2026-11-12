import { Button, Card, Skeleton, toast } from "@heroui/react";
import { useQuery } from "@tanstack/react-query";
import { Download, FileText } from "lucide-react";
import { useState } from "react";
import type { TicketResponse } from "#/api/models";
import { ApiError } from "#/components/ui";
import { useI18n } from "#/i18n";
import { attachmentsQuery, attachmentUrl, deskCatalogQuery, useAttachFiles } from "../hooks";
import { ATTACHMENTS_PER_TICKET, attachmentHint, formatInstant, sizeText } from "../types";
import { AttachmentPicker } from "./AttachmentPicker";

/**
 * The ticket's files: image thumbnails, the rest as links. Images and PDF open in the browser (sandboxed by the
 * server's headers), everything can be downloaded. Any agent adds files in any state, up to 10 per ticket.
 */
export function TicketAttachments({ ticket }: { ticket: TicketResponse }) {
	const i18n = useI18n();
	const { t } = i18n;
	const list = useQuery(attachmentsQuery(ticket.id));
	const catalog = useQuery(deskCatalogQuery());
	const [files, setFiles] = useState<File[]>([]);
	const attach = useAttachFiles(() => {
		setFiles([]);
		toast.success(t("Dodano pliki", "Files added"));
	});
	const room = ATTACHMENTS_PER_TICKET - ticket.attachmentCount;

	return (
		<Card>
			<Card.Header>
				<Card.Title>
					{t("Załączniki", "Attachments")} ({ticket.attachmentCount})
				</Card.Title>
			</Card.Header>
			<Card.Content className="flex flex-col gap-4">
				{list.isPending ? (
					<Skeleton className="h-20 w-full rounded-lg" />
				) : list.isError ? (
					<ApiError error={list.error} />
				) : list.data.length === 0 ? (
					<p className="text-sm text-muted">{t("Brak załączników.", "No attachments.")}</p>
				) : (
					<ul className="grid gap-3 sm:grid-cols-2">
						{list.data.map((a) => {
							const url = attachmentUrl(ticket.id, a.id);
							const image = a.contentType.startsWith("image/");
							return (
								<li key={a.id} className="flex gap-3 rounded-lg border border-separator p-2">
									<a href={url} target="_blank" rel="noreferrer" className="shrink-0">
										{image ? (
											<img
												src={url}
												alt={a.fileName}
												loading="lazy"
												className="size-16 rounded-md object-cover"
											/>
										) : (
											<span className="flex size-16 items-center justify-center rounded-md bg-surface-secondary">
												<FileText className="size-6 text-muted" />
											</span>
										)}
									</a>
									<div className="flex min-w-0 flex-1 flex-col">
										<a
											href={a.inline ? url : attachmentUrl(ticket.id, a.id, true)}
											target={a.inline ? "_blank" : undefined}
											rel="noreferrer"
											className="truncate text-sm font-medium text-foreground hover:underline"
										>
											{a.fileName}
										</a>
										<span className="text-xs text-muted">
											{sizeText(a.size, i18n)} · {a.addedBy}
										</span>
										<span className="text-xs text-muted">{formatInstant(a.at, i18n)}</span>
									</div>
									<a
										href={attachmentUrl(ticket.id, a.id, true)}
										aria-label={t(`Pobierz ${a.fileName}`, `Download ${a.fileName}`)}
										className="self-start rounded-md p-1.5 text-muted hover:bg-surface-secondary hover:text-foreground"
									>
										<Download className="size-4" />
									</a>
								</li>
							);
						})}
					</ul>
				)}

				{room > 0 && catalog.data ? (
					<>
						<AttachmentPicker
							files={files}
							onChange={setFiles}
							extensions={catalog.data.attachmentExtensions}
							room={room}
							hint={attachmentHint(ticket.categoryKey, i18n)}
						/>
						<ApiError error={attach.error} />
						{files.length > 0 ? (
							<div className="flex justify-end">
								<Button
									isPending={attach.isPending}
									onPress={() => attach.mutate({ id: ticket.id, files })}
								>
									{t(`Dodaj pliki (${files.length})`, `Add files (${files.length})`)}
								</Button>
							</div>
						) : null}
					</>
				) : room <= 0 ? (
					<p className="text-xs text-muted">
						{t("Osiągnięto limit 10 załączników.", "The limit of 10 attachments is reached.")}
					</p>
				) : null}
			</Card.Content>
		</Card>
	);
}
