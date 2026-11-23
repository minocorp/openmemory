import { expect, type Page, test } from "@playwright/test";

import { adminRecoveryCode, openAdminPage } from "./support/admin-session";
import { apiGet, apiSend } from "./support/api";
import { addVirtualAuthenticator } from "./support/authenticator";
import { simplePdf } from "./support/pdf";
import { shoot } from "./support/screenshot";

const bodyText =
  "## Terms\n\nMonthly **fee** is 450,000.  \n\n- [ ] send the invoice\n- plain item\n";

async function pageContent(page: Page, path: string): Promise<string> {
  const { data } = await apiGet(page, `/api/pages?path=${encodeURIComponent(path)}`);
  return String(data["content"]);
}

test.describe.serial("memory web", () => {
  let page: Page;
  let authenticator: Awaited<ReturnType<typeof openAdminPage>>["authenticator"];
  const recoveryCode = adminRecoveryCode();

  test.beforeAll(async ({ browser }) => {
    ({ page, authenticator } = await openAdminPage(browser));
    await page.goto("/");
    await shoot(page, "04-home");
  });

  test.afterAll(async () => {
    await page.close();
  });

  test("creates a page at a path and opens it by URL", async () => {
    await page.getByRole("button", { name: "New page" }).click();
    await page.getByLabel("Path").fill("customers/Acme");
    await shoot(page, "05-create-dialog");
    await page.getByRole("button", { name: "Create" }).click();
    await expect(page).toHaveURL(/\/p\/customers\/Acme$/);
    await expect(page.getByLabel("Page title")).toHaveValue("Acme");
    await expect(page.getByRole("link", { name: "Acme" })).toHaveAttribute("aria-current", "page");
    await page.reload();
    await expect(page.getByLabel("Page title")).toHaveValue("Acme");
  });

  test("edits the body and stores the text byte for byte", async () => {
    const editor = page.getByRole("textbox", { name: "Page body" });
    await editor.click();
    await editor.fill(bodyText);
    await page.getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Unsaved changes")).toBeHidden();
    expect(await pageContent(page, "customers/Acme")).toBe(bodyText);
    await page.getByRole("heading", { name: "Acme" }).click();
    await shoot(page, "06-page-body");
  });

  test("adds and edits fields", async () => {
    await page.getByRole("button", { name: "Add field" }).click();
    await shoot(page, "19-field-form");
    await page.getByLabel("Field name").fill("status");
    await page.getByLabel("Field value").fill("active");
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await expect(page.getByRole("button", { name: "Edit status" })).toHaveText("active");
    await page.getByRole("button", { name: "Add field" }).click();
    await page.getByLabel("Field name").fill("renewal");
    await page.getByLabel("Field value").fill("2027-03-01");
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await page.getByRole("button", { name: "Edit status" }).click();
    await page.getByLabel("status", { exact: true }).fill("paused");
    await page.getByLabel("status", { exact: true }).press("Enter");
    await expect(page.getByRole("button", { name: "Edit status" })).toHaveText("paused");
    const content = await pageContent(page, "customers/Acme");
    expect(content).toBe(`---\nstatus: paused\nrenewal: 2027-03-01\n---\n${bodyText}`);
    await shoot(page, "07-fields");
  });

  test("adds timeline lines and opens a source file beside the page", async () => {
    await apiSend(page, {
      method: "POST",
      path: "/api/files?path=contracts/acme.pdf",
      contentType: "application/pdf",
      body: simplePdf([["Service agreement", "Article 3 monthly fee of 500,000 won"], ["Annex"]]),
    });
    await page.getByRole("button", { name: "Add to timeline" }).click();
    await shoot(page, "18-timeline-form");
    await page.getByLabel("Timeline text").fill("Agreed to lower the fee");
    await page.getByLabel("Timeline source").fill("call");
    await page.getByRole("button", { name: "Add line" }).click();
    await expect(page.getByText("Agreed to lower the fee")).toBeVisible();
    await page.getByRole("button", { name: "Add to timeline" }).click();
    await page.getByLabel("Timeline text").fill("Met in person");
    await page.getByLabel("Timeline date").fill("2999-01-01");
    await page.getByRole("button", { name: "Add line" }).click();
    await expect(page.getByText("That date is after today.")).toBeVisible();
    await page.getByLabel("Timeline date").fill("2026-01-15");
    await page.getByRole("button", { name: "Add line" }).click();
    await expect(page.getByText("Met in person")).toBeVisible();
    await expect(page.getByText("2026-01-15", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Add to timeline" }).click();
    await page.getByLabel("Timeline text").fill("Contract signed at 500,000");
    await page.getByLabel("Timeline source").fill('contracts/acme.pdf#p1 "monthly fee of 500,000"');
    await page.getByRole("button", { name: "Add line" }).click();
    const chip = page.getByRole("button", { name: /acme\.pdf p\.1/ });
    await expect(chip).toBeVisible();
    await chip.click();
    await expect(page).toHaveURL(/file=contracts%2Facme\.pdf/);
    const viewer = page.getByRole("region", { name: "File contracts/acme.pdf" });
    await expect(viewer).toBeVisible();
    await expect(viewer.locator(".textLayer span").first()).toBeVisible();
    await expect.poll(() => page.evaluate(() => CSS.highlights.has("source-quote"))).toBe(true);
    await shoot(page, "08-source-chip");
    await page.getByRole("button", { name: "Close file" }).click();
    await expect(viewer).toBeHidden();
    const content = await pageContent(page, "customers/Acme");
    expect(content).toMatch(
      /\n\d{4}-\d{2}-\d{2} \| contracts\/acme\.pdf#p1 "monthly fee of 500,000" — Contract signed at 500,000\n/,
    );
  });

  test("lists versions, shows a diff and restores an older version", async () => {
    await page.getByRole("button", { name: "History" }).click();
    const drawer = page.getByRole("dialog", { name: "History" });
    await expect(drawer.getByText("Current")).toBeVisible();
    await drawer.getByRole("button", { name: /Show changes in version 2/ }).click();
    await shoot(page, "09-history");
    await drawer.getByRole("button", { name: "Restore version 2" }).click();
    await expect(page.getByRole("dialog", { name: "Restore version 2?" })).toBeVisible();
    await expect(
      page.getByRole("dialog", { name: "Restore version 2?" }).getByText("removed line").first(),
    ).toBeVisible();
    await shoot(page, "10-restore-confirm");
    await page.getByRole("button", { name: "Restore", exact: true }).click();
    await expect(page.getByText("Restored version 2")).toBeVisible();
    await expect.poll(() => pageContent(page, "customers/Acme")).toBe(bodyText);
    await page.keyboard.press("Escape");
  });

  test("shows both versions when the page changed under an edit", async () => {
    const editor = page.getByRole("textbox", { name: "Page body" });
    await editor.click();
    await editor.fill(`${bodyText}Mine line.\n`);
    const { data, etag } = await apiGet(page, "/api/pages?path=customers/Acme");
    await apiSend(page, {
      method: "PUT",
      path: `/api/pages/${String(data["id"])}`,
      headers: { "If-Match": etag },
      json: { content: `${bodyText}Theirs line.\n` },
    });
    await page.getByRole("button", { name: "Save" }).click();
    const dialog = page.getByRole("dialog", { name: "This page changed while you were editing" });
    await expect(dialog.getByRole("region", { name: "Their version" })).toContainText(
      "Theirs line.",
    );
    await expect(dialog.getByRole("region", { name: "Your version" })).toContainText("Mine line.");
    await shoot(page, "11-conflict");
    await dialog.getByRole("button", { name: "Keep my version" }).click();
    await expect(dialog).toBeHidden();
    expect(await pageContent(page, "customers/Acme")).toBe(`${bodyText}Mine line.\n`);
  });

  test("writes with the slash menu, a link and the selection toolbar", async () => {
    await page.getByRole("button", { name: "New page" }).click();
    await page.getByLabel("Path").fill("scratch/Editor");
    await page.getByRole("button", { name: "Create" }).click();
    await expect(page.getByLabel("Page title")).toHaveValue("Editor");
    const editor = page.getByRole("textbox", { name: "Page body" });
    await editor.click();
    await page.keyboard.type("/");
    await page.getByRole("option", { name: /Heading 2/ }).click();
    await page.keyboard.type("Title");
    await page.keyboard.press("Enter");
    await page.keyboard.type("/");
    await expect(page.getByRole("listbox")).toBeVisible();
    await shoot(page, "16-slash-menu");
    await page.getByRole("option", { name: /Checklist/ }).click();
    await page.keyboard.type("task one");
    await page.keyboard.press("Enter");
    await page.keyboard.type("[[acme");
    await page.getByRole("option", { name: /contracts\/acme\.pdf/ }).click();
    await page.locator(".cm-task-checkbox").first().click();
    await page.locator(".cm-heading-2").dblclick();
    await shoot(page, "17-selection-toolbar");
    await page.getByRole("button", { name: "Bold" }).click();
    await page.getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Unsaved changes")).toBeHidden();
    expect(await pageContent(page, "scratch/Editor")).toBe(
      "## **Title**\n- [x] task one\n- [ ] [[contracts/acme.pdf]]",
    );
  });

  test("shows what another person changed since the last visit only on request", async ({
    browser,
  }) => {
    await page.goto("/settings?tab=people");
    await page.getByRole("button", { name: "New invite" }).click();
    const link = await page.getByLabel("Invite link").inputValue();
    const other = await browser.newPage();
    await addVirtualAuthenticator(other);
    await other.goto(link);
    await other.getByLabel("Username").fill("sam");
    await other.getByLabel("Display name").fill("Sam");
    await other.getByRole("button", { name: "Create passkey" }).click();
    await other.getByLabel("I saved it").check();
    await other.getByRole("button", { name: "Continue" }).click();
    await expect(other.getByRole("navigation", { name: "Pages and files" })).toBeVisible();
    const { data, etag } = await apiGet(other, "/api/pages?path=customers/Acme");
    await apiSend(other, {
      method: "PATCH",
      path: `/api/pages/${String(data["id"])}`,
      headers: { "If-Match": etag },
      json: { replace: { old: "Mine line.", new: "Edited by sam." } },
    });
    await other.close();
    await page.goto("/p/customers/Acme");
    const indicator = page.getByRole("button", { name: /View changes/ });
    await expect(indicator).toContainText("sam");
    await expect(page.getByRole("region", { name: "Changed since you last looked" })).toBeHidden();
    await expect(page.locator(".cm-changed-line")).toHaveCount(0);
    await shoot(page, "12-page-default");
    await indicator.click();
    const note = page.getByRole("region", { name: "Changed since you last looked" });
    await expect(note).toContainText("sam");
    await expect(page.locator(".cm-changed-line")).toContainText("Edited by sam.");
    await shoot(page, "12-changed-since");
    await note.getByRole("button", { name: "Close" }).click();
    await expect(page.locator(".cm-changed-line")).toHaveCount(0);
    await indicator.click();
    await note.getByRole("button", { name: "Revert" }).click();
    await page.getByRole("button", { name: "Restore", exact: true }).click();
    await expect(page.getByText(/Restored version/)).toBeVisible();
    await expect.poll(() => pageContent(page, "customers/Acme")).toContain("Mine line.");
  });

  test("renames, moves and deletes a page", async () => {
    const title = page.getByLabel("Page title");
    await title.fill("Acme Corp");
    await title.press("Enter");
    await expect(page).toHaveURL(/\/p\/customers\/Acme%20Corp$/);
    await page.getByRole("button", { name: "Page actions" }).click();
    await page.getByRole("menuitem", { name: "Move or rename" }).click();
    await page.getByLabel("Path").fill("clients/Acme Corp");
    await page.getByRole("button", { name: "Move" }).click();
    await expect(page).toHaveURL(/\/p\/clients\/Acme%20Corp$/);
    await expect(page.getByRole("button", { name: /^clients/ })).toBeVisible();
    await page.getByRole("button", { name: "Page actions" }).click();
    await page.getByRole("menuitem", { name: "Delete" }).click();
    await shoot(page, "13-delete-confirm");
    await page.getByRole("button", { name: "Delete", exact: true }).click();
    await expect(page).toHaveURL(/\/$/);
    await page.goto("/p/clients/Acme%20Corp");
    await expect(page.getByRole("heading", { name: "Nothing here yet" })).toBeVisible();
    await shoot(page, "14-missing");
  });

  test("opens a file by its path", async () => {
    await page.goto("/p/contracts/acme.pdf?fpage=1");
    await expect(page.getByRole("region", { name: "File contracts/acme.pdf" })).toBeVisible();
    await page.getByRole("button", { name: "Next page" }).click();
    await expect(page.getByText("2 / 2")).toBeVisible();
    await expect(page.locator(".textLayer").getByText("Annex")).toBeVisible();
    await shoot(page, "20-file");
    const folder = page.getByRole("button", { name: /^contracts/ });
    await expect(folder).toHaveAttribute("aria-expanded", "true");
    await folder.click();
    await expect(folder).toHaveAttribute("aria-expanded", "false");
    await folder.click();
    await expect(folder).toHaveAttribute("aria-expanded", "true");
  });

  test("signs out and back in by typing the username", async () => {
    await page.evaluate(() => {
      PublicKeyCredential.isConditionalMediationAvailable = () => Promise.resolve(false);
    });
    await page.getByRole("button", { name: "Account menu" }).click();
    await page.getByRole("menuitem", { name: "Sign out" }).click();
    await expect(page).toHaveURL(/\/login$/);
    await page.getByLabel("Username").fill("mino");
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page.getByRole("navigation", { name: "Pages and files" })).toBeVisible();
    await page.reload();
  });

  test("signs in from the passkey autofill without typing", async () => {
    await page.getByRole("button", { name: "Account menu" }).click();
    await page.getByRole("menuitem", { name: "Sign out" }).click();
    await expect(page).toHaveURL(/\/login$/);
    await expect(page.getByLabel("Username")).toHaveAttribute("autocomplete", "username webauthn");
    await expect(page.getByLabel("Username")).toHaveValue("");
    await expect(page.getByRole("navigation", { name: "Pages and files" })).toBeVisible();
  });

  test("recovers the account with the recovery code on a new device", async () => {
    await authenticator.remove();
    await addVirtualAuthenticator(page);
    await page.getByRole("button", { name: "Account menu" }).click();
    await page.getByRole("menuitem", { name: "Sign out" }).click();
    await page.getByRole("link", { name: "Lost your passkey?" }).click();
    await expect(page.getByRole("heading", { name: "Recover your account" })).toBeVisible();
    await page.getByLabel("Username").fill("mino");
    await page.getByLabel("Recovery code").fill(recoveryCode);
    await shoot(page, "15-recovery");
    await page.getByRole("button", { name: "Create new passkey" }).click();
    await expect(page.getByLabel("Recovery code")).not.toHaveText(recoveryCode);
    await page.getByLabel("I saved it").check();
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page.getByRole("navigation", { name: "Pages and files" })).toBeVisible();
  });
});
