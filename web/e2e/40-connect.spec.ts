import { expect, type Page, test } from "@playwright/test";
import { z } from "zod";

import { openAdminPage, phoneViewport } from "./support/admin-session";
import {
  approveConsent,
  authorizeUrl,
  agentFor,
  captureCallback,
  createPkce,
  exchangeCode,
  registerClient,
} from "./support/agent";
import { apiSend } from "./support/api";
import { webOrigin } from "./support/config";
import { shoot } from "./support/screenshot";

const device = z.object({ device_code: z.string(), user_code: z.string() });
const tokenReply = z.object({ access_token: z.string() });

async function seedNote(page: Page, path: string) {
  await apiSend(page, {
    method: "POST",
    path: `/api/files?path=${encodeURIComponent(path)}`,
    contentType: "text/plain",
    body: Buffer.from("connect e2e note"),
  });
}

async function startAuthorization(page: Page, clientName: string) {
  const clientId = await registerClient(page, clientName);
  const { verifier, challenge } = createPkce();
  const waitForCallback = await captureCallback(page);
  await page.goto(authorizeUrl({ clientId, challenge, state: "state-1" }));
  return { clientId, verifier, waitForCallback };
}

test.describe.serial("connections", () => {
  let page: Page;

  test.beforeAll(async ({ browser }) => {
    ({ page } = await openAdminPage(browser));
    await seedNote(page, "connect-e2e/in-scope.txt");
    await seedNote(page, "connect-e2e-other/out-of-scope.txt");
  });

  test.afterAll(async () => {
    await page.close();
  });

  test("approves an agent with a name, a folder and read-only access", async () => {
    const { clientId, verifier, waitForCallback } = await startAuthorization(page, "e2e-claude");
    await expect(page.getByRole("heading", { name: "Connect e2e-claude" })).toBeVisible();
    await expect(page.getByText("Unverified client")).toBeVisible();
    await expect(page.getByText("127.0.0.1", { exact: true })).toBeVisible();
    await expect(page.getByLabel("Connection name")).toHaveValue(/e2e-claude/);
    await expect(page.getByLabel("All folders")).toBeChecked();
    await expect(page.getByRole("switch", { name: "Allow writing" })).toBeChecked();
    await shoot(page, "60-approve-form");
    await page.getByLabel("Only these folders").check();
    await page.getByRole("checkbox", { name: "connect-e2e", exact: true }).check();
    await shoot(page, "60-approve-scoped");
    await approveConsent(page, { name: "e2e-claude-code", canWrite: false });
    const callback = await waitForCallback();
    expect(callback.searchParams.get("state")).toBe("state-1");
    const issued = await exchangeCode(page, {
      clientId,
      code: callback.searchParams.get("code") ?? "",
      verifier,
    });
    const agent = agentFor(page, issued.access_token);
    await expect(
      agent.callTool("read", { path: "connect-e2e/in-scope.txt" }),
    ).resolves.toBeTruthy();
    await expect(
      agent.callTool("read", { path: "connect-e2e-other/out-of-scope.txt" }),
    ).rejects.toThrow();
    await expect(
      agent.callTool("write", { path: "connect-e2e/new", content: "text" }),
    ).rejects.toThrow();
    await page.unroute("http://127.0.0.1:9/callback**");
  });

  test("lists the approved connection", async () => {
    const response = await page.request.get("/api/connections");
    const list = z
      .array(z.object({ name: z.string(), can_write: z.boolean(), scopes: z.array(z.string()) }))
      .parse(await response.json());
    expect(list).toContainEqual({
      name: "e2e-claude-code",
      can_write: false,
      scopes: ["connect-e2e"],
    });
  });

  test("denying sends the client an access_denied error", async () => {
    const { waitForCallback } = await startAuthorization(page, "e2e-denied");
    await page.getByRole("button", { name: "Deny" }).click();
    const callback = await waitForCallback();
    expect(callback.searchParams.get("error")).toBe("access_denied");
    expect(callback.searchParams.get("state")).toBe("state-1");
    await page.unroute("http://127.0.0.1:9/callback**");
  });

  test("an expired or unknown request says so", async () => {
    await page.goto("/approve?consent=unknown-token");
    await expect(page.getByRole("heading", { name: "This request has expired" })).toBeVisible();
    await shoot(page, "60-approve-expired");
    await page.goto("/approve");
    await expect(page.getByRole("heading", { name: "This link is incomplete" })).toBeVisible();
  });

  test("approves a device from its code", async () => {
    const clientId = await registerClient(page, "e2e-device");
    const started = await page.request.post(`${webOrigin}/oauth/device`, {
      form: { client_id: clientId, resource: `${webOrigin}/mcp` },
    });
    expect(started.status()).toBe(200);
    const { device_code: deviceCode, user_code: userCode } = device.parse(await started.json());
    await page.goto("/device");
    await page.getByLabel("Code").fill(userCode.toLowerCase());
    await shoot(page, "60-device-code");
    await page.getByRole("button", { name: "Continue" }).click();
    await expect(page.getByRole("heading", { name: "Connect e2e-device" })).toBeVisible();
    await approveConsent(page, { name: "e2e-device-name" });
    await expect(page.getByRole("heading", { name: "Connected" })).toBeVisible();
    const polled = await page.request.post(`${webOrigin}/oauth/token`, {
      form: {
        grant_type: "urn:ietf:params:oauth:grant-type:device_code",
        client_id: clientId,
        device_code: deviceCode,
        resource: `${webOrigin}/mcp`,
      },
    });
    expect(polled.status()).toBe(200);
    expect(tokenReply.parse(await polled.json()).access_token).not.toBe("");
  });

  test("an unknown device code says so", async () => {
    await page.goto("/device?user_code=BCDF-GHJK");
    await expect(page.getByRole("heading", { name: "This request has expired" })).toBeVisible();
  });

  test("the approval screen fits a phone", async ({ browser }) => {
    const { page: phone } = await openAdminPage(browser, phoneViewport);
    const { waitForCallback } = await startAuthorization(phone, "e2e-phone");
    await expect(phone.getByRole("button", { name: "Approve" })).toBeVisible();
    await shoot(phone, "61-approve-phone");
    await phone.getByRole("button", { name: "Deny" }).click();
    await waitForCallback();
    await phone.close();
  });
});
