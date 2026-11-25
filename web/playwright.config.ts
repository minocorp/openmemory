import { defineConfig } from "@playwright/test";

import { uniqueClientHeaders } from "./e2e/support/client-address";
import { apiOrigin, embedded, webOrigin, webPort } from "./e2e/support/config";

export default defineConfig({
  testDir: "./e2e",
  globalSetup: "./e2e/global-setup.ts",
  workers: 1,
  fullyParallel: false,
  forbidOnly: Boolean(process.env["CI"]),
  timeout: 60_000,
  use: {
    baseURL: webOrigin,
    extraHTTPHeaders: uniqueClientHeaders(),
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  webServer: embedded
    ? []
    : {
        command: `pnpm dev --port ${webPort} --strictPort`,
        url: webOrigin,
        reuseExistingServer: false,
        env: { OPENMEMORY_API_ORIGIN: apiOrigin },
      },
});
