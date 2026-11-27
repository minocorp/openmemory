import { expect, type Page, test } from "@playwright/test";

import { openAdminPage } from "./support/admin-session";
import { apiGet, apiSend } from "./support/api";

const path = "live-e2e/Notes";

async function editElsewhere(page: Page, old: string, replacement: string) {
  const { data, etag } = await apiGet(page, `/api/pages?path=${encodeURIComponent(path)}`);
  await apiSend(page, {
    method: "PATCH",
    path: `/api/pages/${String(data["id"])}`,
    headers: { "If-Match": etag },
    json: { replace: { old, new: replacement } },
  });
}

test.describe.serial("live changes", () => {
  let page: Page;

  test.beforeAll(async ({ browser }) => {
    ({ page } = await openAdminPage(browser));
    await apiSend(page, {
      method: "PUT",
      path: `/api/pages?path=${encodeURIComponent(path)}`,
      headers: { "If-None-Match": "*" },
      json: { content: "First line.\n\nMiddle line.\n\nLast line.\n" },
    });
  });

  test.afterAll(async () => {
    await page.close();
  });

  test("an open page shows an edit made elsewhere without a reload", async () => {
    await page.goto(`/p/${path}`);
    const editor = page.getByRole("textbox", { name: "Page body" });
    await expect(editor).toContainText("Middle line.");

    await editElsewhere(page, "Middle line.", "Middle line, edited elsewhere.");

    await expect(editor).toContainText("Middle line, edited elsewhere.");
    await expect(page.getByText("Unsaved changes")).toBeHidden();
  });

  test("an unsaved edit survives an edit elsewhere to other lines", async () => {
    const editor = page.getByRole("textbox", { name: "Page body" });
    await editor.getByText("First line.").click();
    await page.keyboard.press("End");
    await page.keyboard.type(" Mine.");
    await expect(page.getByText("Unsaved changes")).toBeVisible();

    await editElsewhere(page, "Last line.", "Last line, theirs.");

    await expect(editor).toContainText("Last line, theirs.");
    await expect(editor).toContainText("First line. Mine.");
    await page.getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Unsaved changes")).toBeHidden();

    const { data } = await apiGet(page, `/api/pages?path=${encodeURIComponent(path)}`);
    expect(data["content"]).toBe(
      "First line. Mine.\n\nMiddle line, edited elsewhere.\n\nLast line, theirs.\n",
    );
  });
});
