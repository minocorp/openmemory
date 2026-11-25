import { expect, test } from "@playwright/test";

import { openAdminPage, phoneViewport } from "./support/admin-session";
import { apiGet, apiSend } from "./support/api";
import { simplePdf } from "./support/pdf";
import { shoot } from "./support/screenshot";

test.describe.serial("shell", () => {
  test("purges a page and a file after a passkey confirmation", async ({ browser }) => {
    const { page } = await openAdminPage(browser);
    await apiSend(page, {
      method: "PUT",
      path: "/api/pages?path=shell-e2e/Purge",
      headers: { "If-None-Match": "*" },
      json: { content: "Remove me completely.\n" },
    });
    await apiSend(page, {
      method: "POST",
      path: "/api/files?path=shell-e2e/purge.txt",
      contentType: "text/plain",
      body: Buffer.from("remove this file"),
    });
    await page.goto("/p/shell-e2e/Purge");
    await page.getByRole("button", { name: "Page actions" }).click();
    await page.getByRole("menuitem", { name: "Purge permanently" }).click();
    await shoot(page, "75-purge-confirm");
    await page.getByRole("button", { name: "Purge with passkey" }).click();
    await expect(page).toHaveURL(/\/$/);
    expect((await page.request.get("/api/pages?path=shell-e2e/Purge")).status()).toBe(404);
    await page.goto("/p/shell-e2e/purge.txt");
    await page.getByRole("button", { name: "File actions" }).click();
    await page.getByRole("menuitem", { name: "Purge permanently" }).click();
    await page.getByRole("button", { name: "Purge with passkey" }).click();
    await expect(page).toHaveURL(/\/$/);
    expect((await page.request.get("/api/files?path=shell-e2e/purge.txt")).status()).toBe(404);
    await page.context().close();
  });

  test("warns before leaving a page with unsaved edits", async ({ browser }) => {
    const { page } = await openAdminPage(browser);
    await page.goto("/");
    await page.getByRole("button", { name: "New page" }).click();
    await page.getByLabel("Path").fill("shell-e2e/Draft");
    await page.getByRole("button", { name: "Create" }).click();
    await expect(page.getByLabel("Page title")).toHaveValue("Draft");
    await page.getByRole("textbox", { name: "Page body" }).click();
    await page.keyboard.type("Not saved yet");
    await page.getByRole("link", { name: "Home" }).click();
    const dialog = page.getByRole("alertdialog", { name: "Leave without saving?" });
    await expect(dialog).toBeVisible();
    await shoot(page, "70-leave-warning");
    await dialog.getByRole("button", { name: "Keep editing" }).click();
    await expect(page).toHaveURL(/\/p\/shell-e2e\/Draft$/);
    await page.getByRole("link", { name: "Home" }).click();
    await page.getByRole("button", { name: "Leave" }).click();
    await expect(page).toHaveURL(/\/$/);
    await page.context().close();
  });

  test("keeps the sidebar folders after a reload", async ({ browser }) => {
    const { page } = await openAdminPage(browser);
    await apiSend(page, {
      method: "PUT",
      path: "/api/pages?path=sidebar-e2e/Kept",
      headers: { "If-None-Match": "*" },
      json: { content: "The sidebar remembers this folder.\n" },
    });
    await page.goto("/");
    const folder = page.getByRole("button", { name: /^sidebar-e2e/ });
    await expect(folder).toHaveAttribute("aria-expanded", "false");
    await folder.click();
    await page.reload();
    await expect(folder).toHaveAttribute("aria-expanded", "true");
    await page.context().close();
  });

  test("navigates with the phone tabs and opens the source file full screen", async ({
    browser,
  }) => {
    const { page } = await openAdminPage(browser, phoneViewport);
    await apiSend(page, {
      method: "POST",
      path: "/api/files?path=shell-e2e/contract.pdf",
      contentType: "application/pdf",
      body: simplePdf([["Service agreement", "Monthly fee 500,000"]]),
    });
    await apiSend(page, {
      method: "PUT",
      path: "/api/pages?path=shell-e2e/Phone",
      headers: { "If-None-Match": "*" },
      json: {
        content:
          '---\nstatus: active\n---\nPhone page body.\n\n## Timeline\n2026-03-02 | shell-e2e/contract.pdf#p1 "Monthly fee" — signed\n',
      },
    });
    await page.goto("/");
    const tabs = page.getByRole("navigation", { name: "Sections" });
    await expect(tabs).toBeVisible();
    await expect(page.getByRole("navigation", { name: "Pages and files" })).toBeHidden();
    await shoot(page, "71-phone-home");
    await tabs.getByRole("link", { name: "Pages" }).click();
    await expect(page).toHaveURL(/\/pages$/);
    await page.getByRole("button", { name: /^shell-e2e/ }).click();
    await shoot(page, "72-phone-pages");
    await page.getByRole("link", { name: "Phone" }).click();
    await expect(page.getByLabel("Page title")).toHaveValue("Phone");
    const article = page.locator("article");
    expect((await article.boundingBox())?.width).toBeLessThanOrEqual(phoneViewport.width);
    await expect(page.getByRole("textbox", { name: "Page body" })).toContainText(
      "Phone page body.",
    );
    await shoot(page, "73-phone-page");
    await page.getByRole("button", { name: /contract\.pdf p\.1/ }).click();
    const viewer = page.getByRole("region", { name: "File shell-e2e/contract.pdf" });
    await expect(viewer).toBeVisible();
    const box = await page.getByLabel("Source file").boundingBox();
    expect(box).toMatchObject({ x: 0, y: 0, width: phoneViewport.width });
    expect(box?.height).toBeGreaterThanOrEqual(phoneViewport.height - 1);
    await expect(viewer.locator(".textLayer span").first()).toBeVisible();
    await shoot(page, "74-phone-file-sheet");
    await page.getByRole("button", { name: "Close file" }).click();
    await expect(viewer).toBeHidden();
    await tabs.getByRole("link", { name: "Settings" }).click();
    await expect(page).toHaveURL(/\/settings/);
    const { data } = await apiGet(page, "/api/pages?path=shell-e2e/Phone");
    expect(data["path"]).toBe("shell-e2e/Phone");
    await page.context().close();
  });
});
