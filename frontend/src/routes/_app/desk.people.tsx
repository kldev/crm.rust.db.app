import { createFileRoute } from "@tanstack/react-router";
import { PeoplePage } from "#/features/desk/PeoplePage";

export const Route = createFileRoute("/_app/desk/people")({ component: PeoplePage });
