import type { TeamStageStats } from "#/api/models";
import { STAGES, stageLabel } from "#/features/types";
import { formatMoney } from "#/lib";

/**
 * Projects per stage of one team (indexed view crm.v_team_project_stats). One series of magnitudes: a single
 * ink hue, thin bars from a common baseline, the value written beside each bar in text colour; the title
 * names the series, so no legend. Hover shows the budget of the stage.
 */
export function TeamStats({ stats }: { stats: TeamStageStats[] }) {
	const byStage = new Map(stats.map((s) => [s.stage, s]));
	const total = stats.reduce((sum, s) => sum + s.projectCount, 0);
	const budget = stats.reduce((sum, s) => sum + s.budgetTotal, 0);
	const open = stats
		.filter((s) => ["NEW", "ACCEPTED", "ACTIVE", "ON_HOLD"].includes(s.stage))
		.reduce((sum, s) => sum + s.projectCount, 0);
	const max = Math.max(1, ...stats.map((s) => s.projectCount));

	return (
		<div className="flex flex-col gap-6">
			<div className="grid grid-cols-3 gap-3">
				<Tile label="Projects" value={total.toLocaleString("en")} />
				<Tile label="Open" value={open.toLocaleString("en")} />
				<Tile label="Budget" value={formatMoney(budget)} small />
			</div>
			<div>
				<p className="mb-3 text-sm font-medium">Projects by stage</p>
				<ul className="flex flex-col gap-2.5">
					{STAGES.map((stage) => {
						const s = byStage.get(stage);
						const count = s?.projectCount ?? 0;
						return (
							<li
								key={stage}
								className="grid grid-cols-[84px_1fr_48px] items-center gap-3"
								title={`${stageLabel[stage]}: ${count} projects, ${formatMoney(s?.budgetTotal ?? 0)}`}
							>
								<span className="text-xs text-muted">{stageLabel[stage]}</span>
								<span className="h-2 rounded-r-[4px] bg-surface-secondary">
									<span
										className="block h-2 rounded-r-[4px] bg-ink transition-[width] duration-500"
										style={{ width: `${(count / max) * 100}%` }}
									/>
								</span>
								<span className="figures text-right text-xs text-foreground">{count}</span>
							</li>
						);
					})}
				</ul>
			</div>
		</div>
	);
}

function Tile({ label, value, small }: { label: string; value: string; small?: boolean }) {
	return (
		<div className="rounded-lg bg-surface-secondary px-4 py-3">
			<p className="font-mono text-[10px] tracking-[0.1em] text-muted uppercase">{label}</p>
			<p className={`mt-1 font-display font-semibold ${small ? "text-xl" : "text-3xl"}`}>{value}</p>
		</div>
	);
}
