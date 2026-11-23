import { expect, type Page, test } from "@playwright/test";
import { z } from "zod";

import { openAdminPage, phoneViewport } from "./support/admin-session";
import { connectAgent } from "./support/agent";
import { simplePdf } from "./support/pdf";
import { shoot } from "./support/screenshot";

const linkReply = z.object({ upload_link: z.string() });

test.describe.serial("files", () => {
  let page: Page;

  test.beforeAll(async ({ browser }) => {
    ({ page } = await openAdminPage(browser));
  });

  test.afterAll(async () => {
    await page.close();
  });

  test("uploads with the button into the open folder", async () => {
    await page.goto("/files?folder=files-e2e");
    await expect(page.getByText("No files here yet.")).toBeVisible();
    await shoot(page, "62-files-empty");
    await page.getByLabel("Choose files").setInputFiles({
      name: "terms.pdf",
      mimeType: "application/pdf",
      buffer: simplePdf([["Terms of service"], ["Annex"]]),
    });
    const list = page.getByRole("list", { name: "Files" });
    await expect(list.getByRole("link", { name: /terms\.pdf/ })).toBeVisible();
    await expect(page.getByRole("list", { name: "Uploads" })).toContainText("files-e2e/terms.pdf");
    await shoot(page, "62-files-uploaded");
  });

  test("uploads by dropping a file", async () => {
    const data = await page.evaluateHandle(() => {
      const transfer = new DataTransfer();
      transfer.items.add(new File(["dropped note"], "dropped.txt", { type: "text/plain" }));
      return transfer;
    });
    await page.locator("main > div").dispatchEvent("dragover", { dataTransfer: data });
    await shoot(page, "62-files-dragging");
    await page.locator("main > div").dispatchEvent("drop", { dataTransfer: data });
    await expect(
      page.getByRole("list", { name: "Files" }).getByRole("link", { name: /dropped\.txt/ }),
    ).toBeVisible();
  });

  test("rejects a text that holds a secret and says which pattern", async () => {
    await page.getByLabel("Choose files").setInputFiles({
      name: "key.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("-----BEGIN RSA PRIVATE KEY-----\nabc\n-----END RSA PRIVATE KEY-----\n"),
    });
    await expect(page.getByText(/looks like a secret/)).toBeVisible();
    await shoot(page, "62-files-rejected");
  });

  test("browses folders, filters by name and opens a file", async () => {
    await page.goto("/files");
    await page.getByRole("link", { name: /files-e2e/ }).click();
    await expect(page).toHaveURL(/folder=files-e2e/);
    await page.getByLabel("Filter files").fill("dropped");
    const list = page.getByRole("list", { name: "Files" });
    await expect(list.getByRole("link")).toHaveCount(1);
    await page.getByLabel("Filter files").fill("zzz");
    await expect(page.getByText("No files match.")).toBeVisible();
    await page.getByLabel("Filter files").fill("terms");
    await list.locator(`a[href="/p/files-e2e/terms.pdf"]`).click();
    await expect(page).toHaveURL(/\/p\/files-e2e\/terms\.pdf/);
    await expect(page.getByRole("region", { name: "File files-e2e/terms.pdf" })).toBeVisible();
  });

  test("deletes a file from its menu", async () => {
    await page.goto("/files?folder=files-e2e");
    await page.getByRole("button", { name: "Actions for dropped.txt" }).click();
    await page.getByRole("menuitem", { name: "Delete" }).click();
    await page.getByRole("button", { name: "Delete", exact: true }).click();
    await expect(page.getByRole("link", { name: /dropped\.txt/ })).toBeHidden();
  });

  test("a one-time upload link works once without a session", async ({ browser }) => {
    const agent = await connectAgent(page, { name: "e2e-uploader" });
    const reply = linkReply.parse(
      await agent.callTool("upload", { path: "files-e2e/from-link.pdf" }),
    );
    const link = new URL(reply.upload_link);
    const context = await browser.newContext({ baseURL: `${link.protocol}//${link.host}` });
    const visitor = await context.newPage();
    await visitor.goto(link.pathname);
    await expect(visitor.getByRole("heading", { name: "Upload a file" })).toBeVisible();
    await expect(visitor.getByText("files-e2e/from-link.pdf")).toBeVisible();
    await shoot(visitor, "64-upload-link");
    await visitor.getByLabel("Choose file").setInputFiles({
      name: "whatever.pdf",
      mimeType: "application/pdf",
      buffer: simplePdf([["Large scan"]]),
    });
    await expect(visitor.getByRole("heading", { name: "Uploaded" })).toBeVisible();
    await shoot(visitor, "64-upload-link-done");
    await visitor.reload();
    await expect(
      visitor.getByRole("heading", { name: "This link has expired or was already used" }),
    ).toBeVisible();
    await shoot(visitor, "64-upload-link-used");
    await context.close();
    await page.goto("/files?folder=files-e2e");
    await expect(page.getByRole("link", { name: /from-link\.pdf/ })).toBeVisible();
  });

  test("the files screen fits a phone", async ({ browser }) => {
    const { page: phone } = await openAdminPage(browser, phoneViewport);
    await phone.goto("/files?folder=files-e2e");
    await expect(phone.getByRole("link", { name: /terms\.pdf/ })).toBeVisible();
    await shoot(phone, "63-files-phone");
    const width = await phone.evaluate(() => document.documentElement.scrollWidth);
    expect(width).toBeLessThanOrEqual(phoneViewport.width);
    await phone.close();
  });
});
