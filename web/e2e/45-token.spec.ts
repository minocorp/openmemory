import { expect, request as playwrightRequest, type Page, test } from "@playwright/test";

import { openAdminPage } from "./support/admin-session";
import { uniqueClientHeaders } from "./support/client-address";
import { webOrigin } from "./support/config";
import { shoot } from "./support/screenshot";

test.describe.serial("personal access tokens", () => {
  let page: Page;
  let token = "";

  test.beforeAll(async ({ browser }) => {
    ({ page } = await openAdminPage(browser));
  });

  test.afterAll(async () => {
    await page.close();
  });

  test("creates a token with a passkey and shows it once", async () => {
    await page.goto("/settings?tab=connections");
    await page.getByRole("button", { name: "New token" }).click();
    const form = page.getByRole("dialog", { name: "New personal access token" });
    await form.getByLabel("Name").fill("ci-runner");
    await form.getByLabel("Expires after").selectOption("90");
    await shoot(page, "55-token-form");
    await form.getByRole("button", { name: "Create with passkey" }).click();
    const reveal = page.getByRole("dialog", { name: "Copy your token" });
    await expect(reveal).toBeVisible();
    token = (await reveal.getByLabel("Personal access token", { exact: true }).innerText()).trim();
    expect(token).toMatch(/^mpat_/);
    await shoot(page, "55-token-created");
    await reveal.getByRole("button", { name: "Done" }).click();
    await expect(reveal).toBeHidden();
    const row = page
      .getByRole("list", { name: "Connections" })
      .getByRole("listitem")
      .filter({ hasText: "ci-runner" });
    await expect(row).toContainText("Personal access token");
    await expect(row).toContainText("expires in");
    await expect(page.getByText(token)).toHaveCount(0);
    await shoot(page, "55-token-listed");
  });

  test("the token authenticates the api and stops working once revoked", async () => {
    const api = await playwrightRequest.newContext({
      baseURL: webOrigin,
      extraHTTPHeaders: { ...uniqueClientHeaders(), Authorization: `Bearer ${token}` },
    });
    expect((await api.get("/api/tree")).status()).toBe(200);
    expect((await api.get("/api/users")).status()).toBe(403);
    await page.goto("/settings?tab=connections");
    await page.getByRole("button", { name: "Revoke ci-runner" }).click();
    await page.getByRole("button", { name: "Revoke", exact: true }).click();
    await expect(page.getByText("ci-runner")).toHaveCount(0);
    expect((await api.get("/api/tree")).status()).toBe(401);
    await api.dispose();
  });
});
