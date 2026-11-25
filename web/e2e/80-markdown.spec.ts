import { expect, type Page, test } from "@playwright/test";

import { openAdminPage } from "./support/admin-session";
import { apiGet, apiSend } from "./support/api";
import { shoot } from "./support/screenshot";

const planPath = "markdown-e2e/Plan";

const body = [
  "# Plan",
  "",
  "Read [the guide](https://example.com/guide), see [[markdown-e2e/Missing]] and `[[not/a/link]]`.",
  "",
  "- first item",
  "  - nested item that is long enough to wrap onto a second line in the editor column, so the wrapped text has to line up under the text and not under the bullet",
  "- [ ] open task",
  "1. numbered",
  "",
  "| Name | Fee |",
  "| :--- | ---: |",
  "| acme | 450000 |",
  "",
  "```ts",
  "const fee = 450000;",
  "```",
  "",
  "Last line.",
].join("\n");

const timeline = "\n\n## Timeline\n2026-11-24 | call — sent **terms** to [[markdown-e2e/Plan]]\n";

async function pageContent(page: Page, path: string): Promise<string> {
  const { data } = await apiGet(page, `/api/pages?path=${encodeURIComponent(path)}`);
  return String(data["content"]);
}

async function save(page: Page) {
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Unsaved changes")).toBeHidden();
}

test.describe.serial("markdown rendering", () => {
  let page: Page;

  test.beforeAll(async ({ browser }) => {
    ({ page } = await openAdminPage(browser));
    await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
    await apiSend(page, {
      method: "PUT",
      path: `/api/pages?path=${encodeURIComponent(planPath)}`,
      headers: { "If-None-Match": "*" },
      json: { content: body + timeline },
    });
  });

  test.afterAll(async () => {
    await page.close();
  });

  test("renders blocks without their markup", async () => {
    await page.goto(`/p/${planPath}`);
    const editor = page.getByRole("textbox", { name: "Page body" });
    await expect(editor.locator(".cm-heading-1")).toHaveText("Plan");
    await expect(editor.locator(".cm-bullet")).toHaveCount(2);
    await expect(editor.locator(".cm-task-checkbox")).toHaveCount(1);
    await expect(editor.getByRole("table")).toBeVisible();
    await expect(editor).not.toContainText("| Name");
    await expect(editor.locator(".cm-code-language")).toHaveText("ts");
    await expect(editor.locator(".cm-wikilink-missing")).toHaveText("markdown-e2e/Missing");
    await expect(editor.locator(".cm-wikilink")).toHaveCount(1);
    await shoot(page, "80-markdown-rendered");
  });

  test("keeps the caret out of hidden block markup", async () => {
    await page.locator(".cm-heading-1").click();
    await page.keyboard.press("Home");
    await page.keyboard.type("A ");
    await page.keyboard.press("End");
    await page.keyboard.press("Enter");
    await page.keyboard.type("Intro");
    await save(page);
    expect(await pageContent(page, planPath)).toContain("# A Plan\nIntro\n");
  });

  test("opens links with a plain click", async () => {
    const popup = page.waitForEvent("popup");
    await page.locator(".cm-link", { hasText: "the guide" }).click();
    const opened = await popup;
    expect(opened.url()).toBe("https://example.com/guide");
    await opened.close();
    await page.locator(".cm-wikilink-missing").click();
    await expect(page).toHaveURL(/\/p\/markdown-e2e\/Missing$/);
    await expect(page.getByText("Nothing here yet")).toBeVisible();
    await page.goBack();
  });

  test("edits a table cell in place, with Korean input", async () => {
    const table = page.getByRole("textbox", { name: "Page body" }).getByRole("table");
    const fee = table.getByRole("cell", { name: "450000" });
    await fee.click();
    await page.keyboard.press("ControlOrMeta+a");
    await page.keyboard.type("500000");
    await page.keyboard.press("Shift+Tab");
    await page.keyboard.press("ControlOrMeta+a");
    const cdp = await page.context().newCDPSession(page);
    for (const step of ["ㅇ", "에", "에ㅇ", "에이"]) {
      await cdp.send("Input.imeSetComposition", {
        text: step,
        selectionStart: step.length,
        selectionEnd: step.length,
      });
    }
    await cdp.send("Input.insertText", { text: "에이" });
    await expect(page.getByRole("toolbar", { name: "Table" })).toBeVisible();
    await shoot(page, "81-markdown-table-editing");
    await save(page);
    expect(await pageContent(page, planPath)).toContain("| 에이 | 500000 |");
  });

  test("grows and reshapes a table from the keyboard and the toolbar", async () => {
    const table = page.getByRole("textbox", { name: "Page body" }).getByRole("table");
    await table.getByRole("cell", { name: "500000" }).click();
    await page.keyboard.press("Tab");
    await page.keyboard.type("solo");
    await page.getByRole("button", { name: "Insert column right" }).click();
    await page.keyboard.type("note");
    await page.getByRole("button", { name: "Align center" }).click();
    await save(page);
    expect(await pageContent(page, planPath)).toContain(
      [
        "| Name |      | Fee    |",
        "| :--- | :--: | -----: |",
        "| 에이 |      | 500000 |",
        "| solo | note |        |",
      ].join("\n"),
    );
    await table.getByRole("cell", { name: "note" }).click();
    await page.getByRole("button", { name: "More table actions" }).click();
    await page.getByRole("menuitem", { name: "Delete column" }).click();
    await save(page);
    expect(await pageContent(page, planPath)).toContain("| solo |        |\n");
  });

  test("copies a code block", async () => {
    await page.locator(".cm-code-copy").click();
    await expect(page.locator(".cm-code-copy")).toHaveText("Copied");
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("const fee = 450000;");
  });

  test("inserts a table from the slash menu and fills it", async () => {
    await page.getByText("Last line.").click();
    await page.keyboard.press("End");
    await page.keyboard.press("Enter");
    await page.keyboard.type("/table");
    await page.getByRole("option", { name: /Table/ }).click();
    await page.keyboard.type("Item");
    await page.keyboard.press("Tab");
    await page.keyboard.type("Qty");
    await page.keyboard.press("Enter");
    await page.keyboard.type("4");
    await save(page);
    expect(await pageContent(page, planPath)).toContain(
      "Last line.\n\n| Item | Qty |     |\n| --- | --- | --- |\n|     | 4   |     |\n|     |     |     |\n",
    );
  });

  test("renders markdown in timeline entries", async () => {
    const entry = page.getByRole("listitem").filter({ hasText: "sent terms" });
    await expect(entry.locator("strong")).toHaveText("terms");
    await entry.getByRole("button", { name: planPath }).click();
    await expect(page).toHaveURL(new RegExp(`/p/${planPath}$`));
  });

  test("previews a markdown file rendered", async () => {
    await apiSend(page, {
      method: "POST",
      path: "/api/files?path=markdown-e2e/notes.md",
      contentType: "text/markdown",
      body: Buffer.from("# Notes\n\n| Day | Topic |\n| --- | --- |\n| Mon | **pricing** |\n"),
    });
    await page.goto("/p/markdown-e2e/notes.md");
    const view = page.getByRole("textbox", { name: "notes.md" });
    await expect(view.getByRole("table")).toBeVisible();
    await expect(view.locator(".cm-heading-1")).toHaveText("Notes");
    await shoot(page, "82-markdown-file");
  });
});
