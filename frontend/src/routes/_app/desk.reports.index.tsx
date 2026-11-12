import { createFileRoute } from "@tanstack/react-router";
import { ReportsPage } from "#/features/desk/reports/ReportsPage";

export const Route = createFileRoute("/_app/desk/reports/")({ component: ReportsPage });
