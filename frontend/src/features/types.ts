/*
 * Labels and colours of the enums of the API. The values mirror the CHECK constraints of the database
 * (V1__schemas_and_tables.sql, V3__teams_and_projects.sql); Record<...> makes the compiler ask for a label
 * when a value is added.
 */
export type ChipColor = "default" | "accent" | "success" | "warning" | "danger";

export const CLIENT_STATUSES = ["LEAD", "ACTIVE", "INACTIVE"] as const;
export type ClientStatus = (typeof CLIENT_STATUSES)[number];
export const clientStatusLabel: Record<ClientStatus, string> = {
	LEAD: "Lead",
	ACTIVE: "Active",
	INACTIVE: "Inactive",
};
export const clientStatusColor: Record<ClientStatus, ChipColor> = {
	LEAD: "warning",
	ACTIVE: "success",
	INACTIVE: "default",
};

export const STAGES = ["NEW", "ACCEPTED", "ACTIVE", "ON_HOLD", "CLOSED", "DECLINED"] as const;
export type Stage = (typeof STAGES)[number];
export const stageLabel: Record<Stage, string> = {
	NEW: "New",
	ACCEPTED: "Accepted",
	ACTIVE: "Active",
	ON_HOLD: "On hold",
	CLOSED: "Closed",
	DECLINED: "Declined",
};
export const stageColor: Record<Stage, ChipColor> = {
	NEW: "accent",
	ACCEPTED: "warning",
	ACTIVE: "success",
	ON_HOLD: "default",
	CLOSED: "default",
	DECLINED: "danger",
};

/**
 * Allowed stage moves - a copy of crm.project_stage_transitions (V3__teams_and_projects.sql), used only to
 * offer the right buttons. The procedure app.project_change_stage is the authority (51902 otherwise).
 */
export const NEXT_STAGES: Record<Stage, Stage[]> = {
	NEW: ["ACCEPTED", "DECLINED"],
	ACCEPTED: ["ACTIVE", "DECLINED"],
	ACTIVE: ["ON_HOLD", "CLOSED"],
	ON_HOLD: ["ACTIVE", "CLOSED"],
	CLOSED: [],
	DECLINED: [],
};
export const stageAction: Record<Stage, string> = {
	NEW: "Reopen",
	ACCEPTED: "Accept",
	ACTIVE: "Start",
	ON_HOLD: "Put on hold",
	CLOSED: "Close",
	DECLINED: "Decline",
};
export const isOpenStage = (stage: string) =>
	["NEW", "ACCEPTED", "ACTIVE", "ON_HOLD"].includes(stage);

export const ROLES = ["ADMIN", "MANAGER", "SALES"] as const;
export type RoleName = (typeof ROLES)[number];
export const roleLabel: Record<RoleName, string> = {
	ADMIN: "Admin",
	MANAGER: "Manager",
	SALES: "Sales",
};
export const roleColor: Record<RoleName, ChipColor> = {
	ADMIN: "danger",
	MANAGER: "accent",
	SALES: "default",
};

/** Cities of the seed; the filter offers them, the API accepts any city. */
export const CITIES = [
	"Warszawa",
	"Kraków",
	"Łódź",
	"Wrocław",
	"Poznań",
	"Gdańsk",
	"Szczecin",
	"Bydgoszcz",
	"Lublin",
	"Białystok",
	"Katowice",
	"Gdynia",
	"Częstochowa",
	"Radom",
	"Rzeszów",
	"Toruń",
	"Sosnowiec",
	"Kielce",
	"Gliwice",
	"Olsztyn",
];
