import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { type Browser, type BrowserContext, chromium, expect, type Page } from "@playwright/test";
import { z } from "zod";

import { addVirtualAuthenticator, type Credential, resetSignCount } from "./authenticator";
import { uniqueClientHeaders } from "./client-address";
import { runDirectory, webOrigin } from "./config";
import { shoot } from "./screenshot";

type Viewport = { width: number; height: number };

type StoredAdmin = {
  recoveryCode: string;
  storageState: Awaited<ReturnType<BrowserContext["storageState"]>>;
  credentials: Credential[];
};

const instance = process.env["E2E_INSTANCE"] ?? "0";
const adminFile = resolve(runDirectory, `e2e-admin-${instance}.json`);

export const desktopViewport: Viewport = { width: 1280, height: 800 };
export const phoneViewport: Viewport = { width: 390, height: 844 };

export async function bootstrapAdmin(token: string) {
  const browser = await chromium.launch();
  const context = await browser.newContext({
    baseURL: webOrigin,
    viewport: desktopViewport,
    extraHTTPHeaders: uniqueClientHeaders(),
  });
  const page = await context.newPage();
  const authenticator = await addVirtualAuthenticator(page);
  await page.goto(`/bootstrap?token=${token}`);
  await shoot(page, "02-bootstrap");
  await page.getByLabel("Username").fill("mino");
  await page.getByLabel("Display name").fill("Mino");
  await page.getByRole("button", { name: "Create passkey" }).click();
  const code = page.getByLabel("Recovery code");
  await expect(code).toBeVisible();
  const recoveryCode = (await code.innerText()).trim();
  await shoot(page, "03-recovery-code");
  await page.getByLabel("I saved it").check();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(page.getByRole("navigation", { name: "Pages and files" })).toBeVisible();
  const stored: StoredAdmin = {
    recoveryCode,
    storageState: await context.storageState(),
    credentials: await authenticator.credentials(),
  };
  mkdirSync(runDirectory, { recursive: true });
  writeFileSync(adminFile, JSON.stringify(stored));
  resetSignCount();
  await browser.close();
}

export function adminRecoveryCode(): string {
  return readStored().recoveryCode;
}

function readStored(): StoredAdmin {
  return z.custom<StoredAdmin>().parse(JSON.parse(readFileSync(adminFile, "utf8")));
}

export async function openAdminPage(
  browser: Browser,
  viewport: Viewport = desktopViewport,
): Promise<{ page: Page; authenticator: Awaited<ReturnType<typeof addVirtualAuthenticator>> }> {
  const stored = readStored();
  const context = await browser.newContext({
    baseURL: webOrigin,
    viewport,
    storageState: stored.storageState,
    extraHTTPHeaders: uniqueClientHeaders(),
  });
  const page = await context.newPage();
  const authenticator = await addVirtualAuthenticator(page);
  await authenticator.restore(stored.credentials);
  return { page, authenticator };
}
