import { Check } from "lucide-react";
import type { Stage } from "#/features/types";
import { stageLabel } from "#/features/types";

const PATH: Stage[] = ["NEW", "ACCEPTED", "ACTIVE", "CLOSED"];

/** The happy path NEW → ACCEPTED → ACTIVE → CLOSED; ON_HOLD sits on ACTIVE, DECLINED ends the path. */
export function StagePath({ stage }: { stage: Stage }) {
	const position = stage === "ON_HOLD" ? 2 : stage === "DECLINED" ? -1 : PATH.indexOf(stage);
	return (
		<ol className="flex items-center gap-2 overflow-x-auto" aria-label="Project stage">
			{PATH.map((step, i) => {
				const done = position > i || stage === "CLOSED";
				const current = position === i && stage !== "CLOSED";
				return (
					<li key={step} className="flex items-center gap-2">
						<span
							className={[
								"flex items-center gap-2 rounded-full border px-3 py-1 text-xs whitespace-nowrap",
								done ? "border-ink bg-ink text-champagne" : "",
								current ? "border-ink bg-accent-soft font-semibold text-foreground" : "",
								!done && !current ? "border-border text-muted" : "",
							].join(" ")}
						>
							{done ? <Check className="size-3" /> : <span className="font-mono">{i + 1}</span>}
							{current && stage === "ON_HOLD" ? `${stageLabel.ACTIVE} · on hold` : stageLabel[step]}
						</span>
						{i < PATH.length - 1 ? <span className="h-px w-6 bg-border" /> : null}
					</li>
				);
			})}
			{stage === "DECLINED" ? (
				<li className="ml-2 rounded-full border border-danger px-3 py-1 text-xs font-semibold text-danger">
					Declined
				</li>
			) : null}
		</ol>
	);
}
