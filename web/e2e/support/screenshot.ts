import { resolve } from "node:path";

import type { Page } from "@playwright/test";

import { screenshotDirectory } from "./config";

export async function shoot(page: Page, name: string) {
  await page.screenshot({
    path: resolve(screenshotDirectory, `${name}.png`),
    animations: "disabled",
  });
}
