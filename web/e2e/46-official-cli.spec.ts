import { expect, test } from "@playwright/test";

import { openAdminPage } from "./support/admin-session";
import { captureCallback, createPkce } from "./support/agent";
import { webOrigin } from "./support/config";
import { shoot } from "./support/screenshot";

test("the consent screen presents memory-cli as the official CLI", async ({ browser }) => {
  const { page } = await openAdminPage(browser);
  const { challenge } = createPkce();
  await captureCallback(page);
  const query = new URLSearchParams({
    response_type: "code",
    client_id: "memory-cli",
    redirect_uri: "http://127.0.0.1:9/callback",
    code_challenge: challenge,
    code_challenge_method: "S256",
    state: "state-cli",
    resource: `${webOrigin}/api`,
  });
  await page.goto(`${webOrigin}/oauth/authorize?${query.toString()}`);
  await expect(page.getByRole("heading", { name: "Connect Memory CLI" })).toBeVisible();
  await expect(page.getByText("Official memory CLI")).toBeVisible();
  await expect(page.getByText("Unverified client")).toHaveCount(0);
  await expect(page.getByLabel("All folders")).toBeChecked();
  await expect(page.getByRole("switch", { name: "Allow writing" })).toBeChecked();
  await shoot(page, "56-approve-official-cli");
  await page.close();
});
