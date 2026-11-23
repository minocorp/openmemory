import { expect, type Page, test } from "@playwright/test";

import { openAdminPage, phoneViewport } from "./support/admin-session";
import { connectAgent } from "./support/agent";
import { apiGet, apiSend } from "./support/api";
import { shoot } from "./support/screenshot";

test.describe.serial("home", () => {
  let page: Page;

  test.beforeAll(async ({ browser }) => {
    ({ page } = await openAdminPage(browser));
    await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  });

  test.afterAll(async () => {});

  test("shows the first-run panel with copyable setup commands", async () => {
    await page.goto("/");
    const panel = page.getByRole("region", { name: "First run" });
    await expect(panel.getByRole("button", { name: "Create start page" })).toBeVisible();
    await panel.getByText("Claude Code", { exact: true }).click();
    await expect(page.getByLabel("Claude Code Command", { exact: true })).toHaveText(
      /^claude mcp add --transport http memory http:\/\/localhost:\d+\/mcp$/,
    );
    await page.getByRole("button", { name: "Copy Claude Code Command" }).click();
    expect(await page.evaluate(() => navigator.clipboard.readText())).toMatch(
      /^claude mcp add --transport http memory /,
    );
    await panel.getByText("Cursor", { exact: true }).click();
    await panel.getByText("memory CLI", { exact: true }).click();
    await expect(page.getByLabel("memory CLI Command", { exact: true })).toContainText(
      "memory login http://localhost",
    );
    await shoot(page, "30-home-first-run");
  });

  test("groups an agent's changes, shows the diff and reverts the group", async () => {
    const agent = await connectAgent(page, { name: "claude-code" });
    await agent.callTool("write", {
      path: "home-e2e/Acme",
      content: "Fee is 500,000.",
      task: "Update the Acme contract",
    });
    await agent.callTool("write", {
      path: "home-e2e/Beta",
      content: "Beta customer.",
      task: "Update the Acme contract",
    });
    await page.goto("/");
    const group = page.getByRole("button", { name: /Update the Acme contract/ });
    await expect(page.getByRole("listitem").filter({ has: group })).toHaveCount(1);
    await expect(group).toContainText("claude-code");
    await expect(group).toContainText("2 pages");
    await group.click();
    await expect(page.getByText("Fee is 500,000.")).toBeVisible();
    await shoot(page, "30-home-group-open");
    await page.getByRole("button", { name: "Revert group" }).click();
    await expect(page.getByText("Group reverted")).toBeVisible();
    await expect(page.getByText("Reverted", { exact: true })).toBeVisible();
    const missing = await page.request.get("/api/pages?path=home-e2e/Acme");
    expect(missing.status()).toBe(404);
  });

  test("lists the blocking pages when a group cannot be reverted", async () => {
    const agent = await connectAgent(page, { name: "codex" });
    await agent.callTool("write", {
      path: "home-e2e/Gamma",
      content: "First.",
      task: "Add Gamma",
    });
    const { data, etag } = await apiGet(page, "/api/pages?path=home-e2e/Gamma");
    await apiSend(page, {
      method: "PUT",
      path: `/api/pages/${String(data["id"])}`,
      headers: { "If-Match": etag },
      json: { content: "Edited by hand." },
    });
    await page.goto("/");
    await page.getByRole("button", { name: /Add Gamma/ }).click();
    await page.getByRole("button", { name: "Revert group" }).click();
    const alert = page.getByRole("alert");
    await expect(alert).toContainText("Nothing was reverted");
    await expect(alert).toContainText("home-e2e/Gamma");
    await shoot(page, "30-home-revert-blocked");
    await page.getByRole("button", { name: "Mark all seen" }).click();
    await expect(page.getByText("Nothing new.")).toBeVisible();
    await page.reload();
    await expect(page.getByText("Nothing new.")).toBeVisible();
  });

  test("fits a phone screen", async () => {
    const agent = await connectAgent(page, { name: "cursor" });
    await agent.callTool("write", {
      path: "home-e2e/Delta",
      content: "Delta.",
      task: "Add Delta",
    });
    await page.setViewportSize(phoneViewport);
    await page.goto("/");
    await expect(page.getByRole("region", { name: "First run" })).toBeVisible();
    await page.getByRole("button", { name: /Add Delta/ }).click();
    await expect(page.getByRole("button", { name: "Search" })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
      phoneViewport.width,
    );
    await shoot(page, "31-home-phone-group");
    await page.getByRole("link", { name: "Home" }).click();
    await shoot(page, "31-home-phone-first-run");
  });
});
