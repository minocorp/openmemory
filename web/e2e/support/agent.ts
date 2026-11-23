import { createHash, randomBytes } from "node:crypto";

import { expect, type Page } from "@playwright/test";
import { z } from "zod";

import { webOrigin } from "./config";

type ConnectOptions = { name: string; scopes?: string[]; canWrite?: boolean };

export type Agent = {
  accessToken: string;
  callTool(name: string, args: Record<string, unknown>): Promise<unknown>;
};

const callbackPattern = "http://127.0.0.1:9/callback**";
const redirectUri = "http://127.0.0.1:9/callback";
const resource = `${webOrigin}/mcp`;

const registration = z.object({ client_id: z.string() });
const tokens = z.object({ access_token: z.string(), refresh_token: z.string() });
const toolReply = z.object({
  result: z
    .object({
      isError: z.boolean().optional(),
      content: z.array(z.object({ type: z.string(), text: z.string().optional() })),
    })
    .optional(),
  error: z.object({ message: z.string() }).optional(),
});

const base64Url = (bytes: Buffer) => bytes.toString("base64url");

export function createPkce() {
  const verifier = base64Url(randomBytes(32));
  const challenge = base64Url(createHash("sha256").update(verifier).digest());
  return { verifier, challenge };
}

export async function registerClient(page: Page, name: string): Promise<string> {
  const response = await page.request.post(`${webOrigin}/oauth/register`, {
    data: { client_name: name, redirect_uris: [redirectUri], token_endpoint_auth_method: "none" },
  });
  expect(response.status()).toBe(201);
  return registration.parse(await response.json()).client_id;
}

export function authorizeUrl(options: { clientId: string; challenge: string; state: string }) {
  const query = new URLSearchParams({
    response_type: "code",
    client_id: options.clientId,
    redirect_uri: redirectUri,
    code_challenge: options.challenge,
    code_challenge_method: "S256",
    state: options.state,
    resource,
  });
  return `${webOrigin}/oauth/authorize?${query.toString()}`;
}

export async function captureCallback(page: Page): Promise<() => Promise<URL>> {
  let resolveUrl: (url: URL) => void = () => undefined;
  const callback = new Promise<URL>((resolvePromise) => {
    resolveUrl = resolvePromise;
  });
  await page.route(callbackPattern, async (route) => {
    resolveUrl(new URL(route.request().url()));
    await route.fulfill({ status: 200, contentType: "text/plain", body: "callback" });
  });
  return () => callback;
}

export async function exchangeCode(
  page: Page,
  options: { clientId: string; code: string; verifier: string },
) {
  const response = await page.request.post(`${webOrigin}/oauth/token`, {
    form: {
      grant_type: "authorization_code",
      client_id: options.clientId,
      code: options.code,
      redirect_uri: redirectUri,
      code_verifier: options.verifier,
      resource,
    },
  });
  expect(response.status()).toBe(200);
  return tokens.parse(await response.json());
}

export async function approveConsent(
  page: Page,
  options: { name: string; scopes?: string[]; canWrite?: boolean },
) {
  await page.getByLabel("Connection name").fill(options.name);
  if (options.scopes) {
    await page.getByLabel("Only these folders").check();
    for (const folder of options.scopes) {
      await page.getByRole("checkbox", { name: folder, exact: true }).check();
    }
  }
  const writeToggle = page.getByRole("switch", { name: "Allow writing" });
  if (options.canWrite === false) {
    await writeToggle.click();
  }
  await page.getByRole("button", { name: "Approve" }).click();
}

export function agentFor(page: Page, accessToken: string): Agent {
  let nextId = 1;
  return {
    accessToken,
    async callTool(name, args) {
      const response = await page.request.post(`${webOrigin}/mcp`, {
        headers: {
          Authorization: `Bearer ${accessToken}`,
          Accept: "application/json, text/event-stream",
        },
        data: {
          jsonrpc: "2.0",
          id: nextId++,
          method: "tools/call",
          params: { name, arguments: args },
        },
      });
      expect(response.status()).toBe(200);
      const reply = toolReply.parse(parseReply(await response.text()));
      if (reply.error) {
        throw new Error(reply.error.message);
      }
      const text = reply.result?.content.map((block) => block.text ?? "").join("") ?? "";
      if (reply.result?.isError) {
        throw new Error(text);
      }
      return JSON.parse(text);
    },
  };
}

function parseReply(body: string): unknown {
  const dataLine = body.split("\n").find((line) => line.startsWith("data:"));
  return JSON.parse(dataLine ? dataLine.slice("data:".length) : body);
}

export async function connectAgent(adminPage: Page, options: ConnectOptions): Promise<Agent> {
  const clientId = await registerClient(adminPage, options.name);
  const { verifier, challenge } = createPkce();
  const waitForCallback = await captureCallback(adminPage);
  await adminPage.goto(authorizeUrl({ clientId, challenge, state: "e2e" }));
  await approveConsent(adminPage, options);
  const callback = await waitForCallback();
  const code = callback.searchParams.get("code") ?? "";
  expect(code).not.toBe("");
  const issued = await exchangeCode(adminPage, { clientId, code, verifier });
  await adminPage.unroute(callbackPattern);
  return agentFor(adminPage, issued.access_token);
}
