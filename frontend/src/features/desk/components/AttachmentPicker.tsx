import { Button } from "@heroui/react";
import { Paperclip, Upload, X } from "lucide-react";
import { useId, useRef, useState } from "react";
import { useI18n } from "#/i18n";
import { ATTACHMENT_MAX_BYTES, sizeText } from "../types";

/**
 * Files to attach: drag and drop or click to choose. Refuses early what the procedure would refuse (type by
 * extension, empty, over 20 MB, over the room left on the ticket) - informative only, the procedure decides.
 */
export function AttachmentPicker({
	files,
	onChange,
	extensions,
	room,
	hint,
}: {
	files: File[];
	onChange: (files: File[]) => void;
	/** Allowed extensions with the dot, from the catalog. */
	extensions: string[];
	/** How many more files the ticket takes. */
	room: number;
	hint?: string;
}) {
	const i18n = useI18n();
	const { t } = i18n;
	const inputId = useId();
	const input = useRef<HTMLInputElement>(null);
	const [dragging, setDragging] = useState(false);
	const [problems, setProblems] = useState<string[]>([]);

	const add = (list: FileList | null) => {
		if (!list) return;
		const next = [...files];
		const refused: string[] = [];
		for (const file of Array.from(list)) {
			const dot = file.name.lastIndexOf(".");
			const extension = dot < 0 ? "" : file.name.slice(dot).toLowerCase();
			if (!extensions.includes(extension))
				refused.push(
					t(`„${file.name}”: niedozwolony typ pliku`, `“${file.name}”: file type not allowed`),
				);
			else if (file.size === 0)
				refused.push(t(`„${file.name}”: plik jest pusty`, `“${file.name}”: the file is empty`));
			else if (file.size > ATTACHMENT_MAX_BYTES)
				refused.push(t(`„${file.name}”: ponad 20 MB`, `“${file.name}”: over 20 MB`));
			else if (next.length >= room)
				refused.push(
					t(`„${file.name}”: limit ${room} plików`, `“${file.name}”: the limit is ${room} files`),
				);
			else next.push(file);
		}
		setProblems(refused);
		onChange(next);
	};

	return (
		<div className="flex flex-col gap-2">
			<label
				htmlFor={inputId}
				onDragOver={(e) => {
					e.preventDefault();
					setDragging(true);
				}}
				onDragLeave={() => setDragging(false)}
				onDrop={(e) => {
					e.preventDefault();
					setDragging(false);
					add(e.dataTransfer.files);
				}}
				className={`flex cursor-pointer flex-col items-center gap-1 rounded-xl border border-dashed px-4 py-6 text-center transition-colors ${
					dragging ? "border-accent bg-accent-soft" : "border-separator hover:bg-surface-secondary"
				}`}
			>
				<Upload className="size-5 text-muted" />
				<span className="text-sm text-foreground">
					{t(
						"Przeciągnij pliki tutaj albo kliknij, żeby wybrać",
						"Drag files here or click to choose",
					)}
				</span>
				<span className="text-xs text-muted">
					{hint ?? t("Zrzuty ekranu, dokumenty, materiały", "Screenshots, documents, materials")} ·{" "}
					{t(`do 20 MB na plik, najwyżej ${room}`, `up to 20 MB per file, at most ${room}`)}
				</span>
				<input
					ref={input}
					id={inputId}
					type="file"
					multiple
					accept={extensions.join(",")}
					className="sr-only"
					onChange={(e) => {
						add(e.target.files);
						e.target.value = "";
					}}
				/>
			</label>
			{problems.map((p) => (
				<p key={p} className="text-xs text-danger">
					{p}
				</p>
			))}
			{files.length > 0 ? (
				<ul className="flex flex-col gap-1">
					{files.map((file, i) => (
						<li
							key={`${file.name}-${i}`}
							className="flex items-center gap-2 rounded-md bg-surface-secondary px-3 py-1.5 text-sm"
						>
							<Paperclip className="size-3.5 text-muted" />
							<span className="min-w-0 flex-1 truncate">{file.name}</span>
							<span className="text-xs text-muted">{sizeText(file.size, i18n)}</span>
							<Button
								isIconOnly
								size="sm"
								variant="ghost"
								aria-label={t(`Usuń ${file.name}`, `Remove ${file.name}`)}
								onPress={() => onChange(files.filter((_, j) => j !== i))}
							>
								<X className="size-3.5" />
							</Button>
						</li>
					))}
				</ul>
			) : null}
		</div>
	);
}
