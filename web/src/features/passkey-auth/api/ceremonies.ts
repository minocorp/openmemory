import {
  type PublicKeyCredentialCreationOptionsJSON,
  type PublicKeyCredentialRequestOptionsJSON,
  startAuthentication,
  startRegistration,
} from "@simplewebauthn/browser";

import { client, unwrap, unwrapEmpty } from "@/shared/api";
import { z } from "@/shared/lib/zod";

const isObject = (value: unknown): value is object => typeof value === "object" && value !== null;

const jsonObject = z.record(z.string(), z.unknown());

const creationOptions = z.object({
  publicKey: z.custom<PublicKeyCredentialCreationOptionsJSON>(isObject),
});

const requestOptions = z.object({
  publicKey: z.custom<PublicKeyCredentialRequestOptionsJSON>(isObject),
});

export type SignupKind = "bootstrap" | "invite";

export async function signUp(options: {
  kind: SignupKind;
  token: string;
  username: string;
  displayName: string;
}) {
  const body = {
    token: options.token,
    username: options.username,
    display_name: options.displayName,
  };
  const started = await unwrap(
    options.kind === "bootstrap"
      ? client.POST("/api/auth/bootstrap/start", { body })
      : client.POST("/api/auth/register/start", { body }),
  );
  const credential = await startRegistration({
    optionsJSON: creationOptions.parse(started.options).publicKey,
  });
  const finish = { challenge: started.challenge, credential: jsonObject.parse(credential) };
  return unwrap(
    options.kind === "bootstrap"
      ? client.POST("/api/auth/bootstrap/finish", { body: finish })
      : client.POST("/api/auth/register/finish", { body: finish }),
  );
}

export async function signIn(options: { username: string }) {
  const started = await unwrap(client.POST("/api/auth/login/start", { body: options }));
  const credential = await startAuthentication({
    optionsJSON: requestOptions.parse(started.options).publicKey,
  });
  return unwrap(
    client.POST("/api/auth/login/finish", {
      body: { challenge: started.challenge, credential: jsonObject.parse(credential) },
    }),
  );
}

export async function signInWithAutofill() {
  const started = await unwrap(client.POST("/api/auth/login/discoverable/start"));
  const credential = await startAuthentication({
    optionsJSON: requestOptions.parse(started.options).publicKey,
    useBrowserAutofill: true,
  });
  return unwrap(
    client.POST("/api/auth/login/discoverable/finish", {
      body: { challenge: started.challenge, credential: jsonObject.parse(credential) },
    }),
  );
}

export async function recover(options: { username: string; code: string }) {
  const started = await unwrap(client.POST("/api/auth/recovery/start", { body: options }));
  const credential = await startRegistration({
    optionsJSON: creationOptions.parse(started.options).publicKey,
  });
  return unwrap(
    client.POST("/api/auth/recovery/finish", {
      body: { challenge: started.challenge, credential: jsonObject.parse(credential) },
    }),
  );
}

export function signOut() {
  return unwrapEmpty(client.POST("/api/auth/logout"));
}
