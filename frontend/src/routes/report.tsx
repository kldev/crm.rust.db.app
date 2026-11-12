import { createFileRoute } from "@tanstack/react-router";
import { PublicReportPage } from "#/features/desk/PublicReportPage";

// The public report form: no sign-in (ADR-0007).
export const Route = createFileRoute("/report")({ component: PublicReportPage });
