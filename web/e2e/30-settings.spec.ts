import { expect, type Page, test } from "@playwright/test";
import { z } from "zod";

import { openAdminPage, phoneViewport } from "./support/admin-session";
import { connectAgent } from "./support/agent";
import { apiSend } from "./support/api";
import { addVirtualAuthenticator } from "./support/authenticator";
import { webOrigin } from "./support/config";
import { shoot } from "./support/screenshot";

test.describe.serial("settings", () => {
  let page: Page;
  let authenticator: Awaited<ReturnType<typeof openAdminPage>>["authenticator"];

  test.beforeAll(async ({ browser }) => {
    ({ page, authenticator } = await openAdminPage(browser));
  });

  test.afterAll(async () => {
    await page.context().close();
  });

  test("lists passkeys and adds and removes another one", async () => {
    await page.goto("/settings");
    const list = page.getByRole("list", { name: "Passkeys" });
    await expect(list.getByRole("listitem")).toHaveCount(1);
    await shoot(page, "50-settings-account");
    const original = await authenticator.credentials();
    await page.route("**/api/auth/passkeys/start", async (route) => {
      await authenticator.remove();
      authenticator = await addVirtualAuthenticator(page);
      await route.continue();
    });
    await page.getByLabel("Passkey name").fill("Laptop e2e");
    await page.getByRole("button", { name: "Add passkey" }).click();
    await expect(list.getByRole("listitem")).toHaveCount(2);
    await page.unroute("**/api/auth/passkeys/start");
    await expect(list).toContainText("Laptop e2e");
    await page.getByRole("button", { name: "Remove passkey Laptop e2e" }).click();
    await page.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(list.getByRole("listitem")).toHaveCount(1);
    await expect(list).not.toContainText("Laptop e2e");
    await authenticator.remove();
    authenticator = await addVirtualAuthenticator(page);
    await authenticator.restore(original);
  });

  test("refuses to remove the only passkey", async () => {
    const list = page.getByRole("list", { name: "Passkeys" });
    await list.getByRole("button", { name: /^Remove passkey/ }).click();
    await page.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(page.getByText("You cannot remove your only passkey.")).toBeVisible();
    await expect(list.getByRole("listitem")).toHaveCount(1);
  });

  test("edits the display name but keeps the username", async () => {
    await page.goto("/settings");
    await expect(page.getByLabel("Username")).toHaveValue("mino");
    await page.getByLabel("Display name").fill("Mino Kim");
    await page.getByRole("button", { name: "Save profile" }).click();
    await expect(page.getByText("Profile saved")).toBeVisible();
    await expect(page.getByText("Signed in as")).toContainText("Mino Kim");
    await expect(page.getByText("Signed in as")).toContainText("@mino");
    await page.getByLabel("Display name").fill("Mino");
    await page.getByRole("button", { name: "Save profile" }).click();
    await expect(page.getByText("Signed in as")).not.toContainText("Mino Kim");
  });

  test("invites a second person who registers and appears in the list", async ({ browser }) => {
    await page.goto("/settings?tab=people");
    await expect(page.getByText("No open invites.")).toBeVisible();
    await shoot(page, "50-settings-people-empty");
    await page.getByRole("button", { name: "New invite" }).click();
    const link = await page.getByLabel("Invite link").inputValue();
    expect(link).toContain(`${webOrigin}/register?token=`);
    await expect(page.getByRole("list", { name: "Open invites" })).toBeVisible();

    const context = await browser.newContext({ baseURL: webOrigin });
    const guest = await context.newPage();
    await addVirtualAuthenticator(guest);
    await guest.goto(link);
    await guest.getByLabel("Username").fill("sam-settings");
    await guest.getByLabel("Display name").fill("Sam");
    await guest.getByRole("button", { name: "Create passkey" }).click();
    await guest.getByLabel("I saved it").check();
    await guest.getByRole("button", { name: "Continue" }).click();
    await expect(guest.getByRole("navigation", { name: "Pages and files" })).toBeVisible();

    await page.reload();
    await expect(page.getByText("No open invites.")).toBeVisible();
    const people = page.getByRole("list", { name: "People" });
    await expect(people).toContainText("Sam");
    await expect(people).toContainText("@sam-settings");
    await shoot(page, "50-settings-people");

    const admin = page.getByRole("switch", { name: "Admin sam-settings" });
    await expect(admin).not.toBeChecked();
    await admin.click();
    await expect(admin).toBeChecked();
    await expect.poll(() => adminFlag(page, "sam-settings")).toBe(true);
    await admin.click();
    await expect.poll(() => adminFlag(page, "sam-settings")).toBe(false);

    await guest.goto("/settings?tab=people");
    const nav = guest.getByRole("navigation", { name: "Settings" });
    await expect(nav.getByRole("link", { name: "Account" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Connections" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "People" })).toHaveCount(0);
    await expect(nav.getByRole("link", { name: "System" })).toHaveCount(0);
    await expect(guest.getByRole("region", { name: "Passkeys" })).toBeVisible();
    await context.close();
  });

  test("revokes an open invite", async () => {
    await page.goto("/settings?tab=people");
    await page.getByRole("button", { name: "New invite" }).click();
    const invites = page.getByRole("list", { name: "Open invites" });
    await expect(invites.getByRole("listitem")).toHaveCount(1);
    await invites.getByRole("button", { name: /^Revoke invite/ }).click();
    await expect(page.getByText("No open invites.")).toBeVisible();
  });

  test("shows the backup warning and exports and imports a zip", async () => {
    await apiSend(page, {
      method: "PUT",
      path: "/api/pages?path=settings-e2e/export-me",
      headers: { "If-None-Match": "*" },
      json: { content: "Exported text.\n" },
    });
    await page.goto("/settings?tab=system");
    await expect(page.getByRole("alert")).toContainText("No backup is configured.");
    await shoot(page, "50-settings-system");
    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Export" }).click();
    const archive = await (await download).path();
    await page.getByLabel("Import zip").setInputFiles(archive);
    const report = page.getByRole("status");
    await expect(report).toContainText("Imported 0 pages and 0 files.");
    await expect(report).toContainText("settings-e2e/export-me");
    await expect(report).toContainText("path taken");
    await shoot(page, "50-settings-import-report");
  });

  test("edits and revokes a connection", async () => {
    await connectAgent(page, { name: "settings-agent", scopes: ["settings-e2e"] });
    await page.goto("/settings?tab=connections");
    const list = page.getByRole("list", { name: "Connections" });
    const row = list.getByRole("listitem").filter({ hasText: "settings-agent" });
    await expect(row).toContainText("settings-e2e");
    await shoot(page, "50-settings-connections");
    await row.getByRole("button", { name: "Edit settings-agent" }).click();
    const dialog = page.getByRole("dialog", { name: "Edit connection" });
    await shoot(page, "50-settings-connection-edit");
    await dialog.getByLabel("Name").fill("settings-agent-renamed");
    await dialog.getByRole("switch", { name: "Can write" }).click();
    await dialog.getByRole("button", { name: "Save" }).click();
    await expect(dialog).toBeHidden();
    const renamed = list.getByRole("listitem").filter({ hasText: "settings-agent-renamed" });
    await expect(renamed).toContainText("Read only");
    await renamed.getByRole("button", { name: "Revoke settings-agent-renamed" }).click();
    await page.getByRole("button", { name: "Revoke", exact: true }).click();
    await expect(list.getByText("settings-agent-renamed")).toHaveCount(0);
  });

  test("lays out settings on a phone", async ({ browser }) => {
    const { page: phone } = await openAdminPage(browser, phoneViewport);
    await phone.goto("/settings");
    await expect(phone.getByRole("navigation", { name: "Sections" })).toBeVisible();
    await expect(phone.getByRole("region", { name: "Passkeys" })).toBeVisible();
    await shoot(phone, "51-settings-phone-account");
    await phone.getByRole("link", { name: "System" }).click();
    await expect(phone.getByRole("alert")).toBeVisible();
    await shoot(phone, "51-settings-phone-system");
    await phone.getByRole("link", { name: "People" }).click();
    await expect(phone.getByRole("button", { name: "New invite" })).toBeVisible();
    await shoot(phone, "51-settings-phone-people");
    await phone.context().close();
  });
});

const usersReply = z.array(z.object({ username: z.string(), is_admin: z.boolean() }));

async function adminFlag(page: Page, name: string): Promise<boolean | undefined> {
  const response = await page.request.get("/api/users");
  const users = usersReply.parse(await response.json());
  return users.find((user) => user.username === name)?.is_admin;
}
