import { existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import type { Page } from "@playwright/test";
import { z } from "zod";

import { runDirectory } from "./config";

export const credentialSchema = z.object({
  credentialId: z.string(),
  isResidentCredential: z.boolean(),
  rpId: z.string().exactOptional(),
  privateKey: z.string(),
  userHandle: z.string().exactOptional(),
  signCount: z.number(),
  userName: z.string().exactOptional(),
  userDisplayName: z.string().exactOptional(),
});

export type Credential = z.infer<typeof credentialSchema>;

const signCountFile = resolve(runDirectory, `e2e-sign-count-${process.env["E2E_INSTANCE"] ?? "0"}`);
const signCountStep = 1000;

export function resetSignCount() {
  rmSync(signCountFile, { force: true });
}

function nextSignCount(): number {
  const previous = existsSync(signCountFile) ? Number(readFileSync(signCountFile, "utf8")) : 0;
  const next = previous + signCountStep;
  writeFileSync(signCountFile, String(next));
  return next;
}

const options = {
  protocol: "ctap2",
  transport: "internal",
  hasResidentKey: true,
  hasUserVerification: true,
  isUserVerified: true,
  automaticPresenceSimulation: true,
} as const;

export async function addVirtualAuthenticator(page: Page) {
  const session = await page.context().newCDPSession(page);
  await session.send("WebAuthn.enable");
  const { authenticatorId } = await session.send("WebAuthn.addVirtualAuthenticator", { options });
  return {
    remove: () => session.send("WebAuthn.removeVirtualAuthenticator", { authenticatorId }),
    credentials: async (): Promise<Credential[]> => {
      const { credentials } = await session.send("WebAuthn.getCredentials", { authenticatorId });
      return z.array(credentialSchema).parse(credentials);
    },
    restore: async (credentials: Credential[]) => {
      const signCount = nextSignCount();
      for (const credential of credentials) {
        // the server rejects a sign counter that does not grow, so each restored copy starts higher than the last
        await session.send("WebAuthn.addCredential", {
          authenticatorId,
          credential: { ...credential, signCount },
        });
      }
    },
  };
}
