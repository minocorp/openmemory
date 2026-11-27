import { expect, type Page, test } from "@playwright/test";

import { openAdminPage, phoneViewport } from "./support/admin-session";
import { apiSend } from "./support/api";
import { simplePdf } from "./support/pdf";
import { shoot } from "./support/screenshot";

const dayMs = 86_400_000;

const isoDateIn = (days: number) => new Date(Date.now() + days * dayMs).toISOString().slice(0, 10);

async function createPage(page: Page, path: string, content: string) {
  await apiSend(page, {
    method: "PUT",
    path: `/api/pages?path=${encodeURIComponent(path)}`,
    headers: { "If-None-Match": "*" },
    json: { content },
  });
}

test.describe.serial("command palette", () => {
  let page: Page;

  test.beforeAll(async ({ browser }) => {
    ({ page } = await openAdminPage(browser));
    await createPage(
      page,
      "search-e2e/Vendor",
      `---\nrenewal: ${isoDateIn(30)}\n---\nQuarterly invoice rhythm for the vendor.\n`,
    );
    await createPage(
      page,
      "search-e2e/Distant",
      `---\nrenewal: ${isoDateIn(400)}\n---\nQuarterly invoice rhythm for the distant one.\n`,
    );
    await createPage(page, "search-e2e/계약서", "계약 조건은 월 오십만원입니다.\n");
    await apiSend(page, {
      method: "POST",
      path: "/api/files?path=search-e2e/terms.pdf",
      contentType: "application/pdf",
      body: simplePdf([["Cover sheet"], ["Zebra clause nine"]]),
    });
    await page.goto("/");
  });

  test.afterAll(async () => {
    await page.close();
  });

  const palette = () => page.getByRole("dialog", { name: "Search" });
  const input = () => palette().getByRole("combobox", { name: "Search pages and files" });

  test("opens with the keyboard shortcut and shows the syntax hint", async () => {
    await page.keyboard.press("ControlOrMeta+k");
    await expect(input()).toBeFocused();
    await expect(palette().getByText("renewal<+90d")).toBeVisible();
    await shoot(page, "40-search-empty");
    await page.keyboard.press("Escape");
    await expect(palette()).toBeHidden();
  });

  test("opens from the sidebar button and finds a page by its text", async () => {
    await page.getByRole("button", { name: "Search", exact: false }).first().click();
    await input().fill("invoice rhythm");
    const option = palette().getByRole("option", { name: /Vendor/ });
    await expect(option).toBeVisible();
    await expect(option).toContainText("Quarterly invoice rhythm");
    await shoot(page, "40-search-results");
    await option.click();
    await expect(page).toHaveURL(/\/p\/search-e2e\/Vendor$/);
    await expect(palette()).toBeHidden();
  });

  test("finds a Korean page with a two-character query", async () => {
    await page.keyboard.press("ControlOrMeta+k");
    await input().fill("계약");
    await expect(palette().getByRole("option", { name: /계약서/ })).toBeVisible();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/\/p\/search-e2e\/%EA%B3%84%EC%95%BD%EC%84%9C$/);
  });

  test("opens a file hit at its PDF page", async () => {
    await page.keyboard.press("ControlOrMeta+k");
    await input().fill("Zebra");
    const option = palette().getByRole("option", { name: /terms\.pdf/ });
    await expect(option).toContainText("p2");
    await shoot(page, "40-search-file-hit");
    await option.click();
    await expect(page).toHaveURL(/fpage=2/);
    const viewer = page.getByRole("region", { name: "File search-e2e/terms.pdf" });
    await expect(viewer.getByText("2 / 2")).toBeVisible();
  });

  test("filters by a field condition", async () => {
    await page.keyboard.press("ControlOrMeta+k");
    await input().fill("renewal<+90d");
    await expect(palette().getByRole("option", { name: /Vendor/ })).toBeVisible();
    await expect(palette().getByRole("option", { name: /Distant/ })).toHaveCount(0);
    await page.keyboard.press("Escape");
  });

  test("moves through results with the arrow keys", async () => {
    await page.keyboard.press("ControlOrMeta+k");
    await input().fill("Quarterly");
    const options = palette().getByRole("option");
    await expect(options).toHaveCount(2);
    await expect(options.first()).toHaveAttribute("aria-selected", "true");
    await page.keyboard.press("ArrowDown");
    await expect(options.nth(1)).toHaveAttribute("aria-selected", "true");
    await page.keyboard.press("ArrowDown");
    await expect(options.first()).toHaveAttribute("aria-selected", "true");
    await page.keyboard.press("ArrowUp");
    await expect(options.nth(1)).toHaveAttribute("aria-selected", "true");
    await page.keyboard.press("Escape");
  });

  test("groups matches by folder and searches inside one", async () => {
    await createPage(page, "search-e2e/groups/one/Alpha", "Lantern note one.\n");
    await createPage(page, "search-e2e/groups/two/Beta", "Lantern note two.\n");
    await page.keyboard.press("ControlOrMeta+k");
    await input().fill("lantern");
    const one = palette().getByRole("group", { name: "search-e2e/groups/one/" });
    await expect(one.getByRole("option", { name: /Alpha/ })).toBeVisible();
    await expect(palette().getByRole("group", { name: "search-e2e/groups/two/" })).toBeVisible();
    await shoot(page, "40-search-folders");
    await one.getByRole("option", { name: "Search in search-e2e/groups/one 1" }).click();
    await expect(input()).toHaveValue("lantern in:search-e2e/groups/one");
    const matches = palette().getByRole("group", { name: "Matches" });
    await expect(matches.getByRole("option", { name: /Alpha/ })).toBeVisible();
    await expect(palette().getByRole("option", { name: /Beta/ })).toHaveCount(0);
    await page.keyboard.press("Escape");
  });

  test("explains how an empty search was read", async () => {
    await page.keyboard.press("ControlOrMeta+k");
    await input().fill("nothing-matches-this-qqq");
    await expect(palette().getByText("No results for")).toBeVisible();
    await shoot(page, "40-search-no-hits");
    await page.keyboard.press("Escape");
  });

  test("searches from a phone", async ({ browser }) => {
    const { page: phone } = await openAdminPage(browser, phoneViewport);
    await phone.goto("/pages");
    await phone.getByRole("button", { name: "Search", exact: false }).first().click();
    const dialog = phone.getByRole("dialog", { name: "Search" });
    await dialog.getByRole("combobox").fill("invoice");
    await expect(dialog.getByRole("option", { name: /Vendor/ })).toBeVisible();
    await shoot(phone, "41-search-phone-results");
    await dialog.getByRole("option", { name: /Vendor/ }).click();
    await expect(phone).toHaveURL(/\/p\/search-e2e\/Vendor$/);
    await phone.close();
  });
});
