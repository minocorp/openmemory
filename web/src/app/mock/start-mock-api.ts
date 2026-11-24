import { http } from "msw";
import { setupWorker } from "msw/browser";

import { accountHandlers } from "./account-handlers";
import { changeHandlers } from "./change-handlers";
import { fileHandlers } from "./file-handlers";
import { pageHandlers } from "./page-handlers";
import { problem } from "./respond";
import { seed } from "./seed";

export async function startMockApi() {
  seed();
  const worker = setupWorker(
    ...pageHandlers,
    ...fileHandlers,
    ...changeHandlers,
    ...accountHandlers,
    http.all("/api/*", () => problem(501, "not_in_mock")),
  );
  await worker.start({ onUnhandledFrame: "bypass", quiet: true });
}
