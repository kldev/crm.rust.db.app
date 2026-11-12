import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// In development the API runs in docker compose (CRM_HTTP_PORT, default 8088). The proxy keeps one origin, so
// the session cookie and the XSRF-TOKEN cookie work exactly as when Spring serves the built app.
const api = process.env.CRM_API_URL ?? "http://127.0.0.1:8088";

export default defineConfig({
	resolve: { tsconfigPaths: true },
	plugins: [tanstackRouter({ target: "react", autoCodeSplitting: true }), react(), tailwindcss()],
	server: {
		proxy: {
			"/api": { target: api, changeOrigin: false },
			"/v3": { target: api, changeOrigin: false },
		},
	},
	build: { outDir: "dist", emptyOutDir: true, chunkSizeWarningLimit: 900 },
});
