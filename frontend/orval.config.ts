import { defineConfig } from "orval";

// The client is generated from the running API (utoipa, /v3/api-docs): `yarn api:gen` with the compose stack up.
// Plain axios functions; queries and their keys are written by hand in src/api/query-keys and features/*.
const api = process.env.CRM_API_URL ?? "http://127.0.0.1:8088";

export default defineConfig({
	crm: {
		input: { target: `${api}/v3/api-docs` },
		output: {
			mode: "tags-split",
			client: "axios-functions",
			target: "./src/api/endpoints",
			schemas: "./src/api/models",
			clean: true,
			override: { mutator: { path: "./src/api/mutator.ts", name: "customInstance" } },
		},
	},
});
