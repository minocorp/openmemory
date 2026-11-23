import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const apiOrigin = process.env["MEMORY_API_ORIGIN"] ?? "http://localhost:8080";
const proxiedPaths = ["/api", "/mcp", "/oauth", "/.well-known", "/healthz"];

export default defineConfig({
  plugins: [tanstackRouter(), react(), tailwindcss()],
  resolve: { tsconfigPaths: true },
  server: {
    proxy: Object.fromEntries(proxiedPaths.map((path) => [path, apiOrigin])),
  },
  build: { outDir: "dist", emptyOutDir: true, chunkSizeWarningLimit: 600 },
  test: { include: ["src/**/*.test.ts"] },
});
