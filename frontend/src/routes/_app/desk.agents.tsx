import { createFileRoute } from "@tanstack/react-router";
import { AgentsPage } from "#/features/desk/AgentsPage";

export const Route = createFileRoute("/_app/desk/agents")({ component: AgentsPage });
