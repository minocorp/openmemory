import {
  type PublicKeyCredentialCreationOptionsJSON,
  type PublicKeyCredentialRequestOptionsJSON,
  startAuthentication,
  startRegistration,
} from "@simplewebauthn/browser";

import { z } from "@/shared/lib/zod";

const isObject = (value: unknown): value is object => typeof value === "object" && value !== null;

const jsonObject = z.record(z.string(), z.unknown());

const creationOptions = z.object({
  publicKey: z.custom<PublicKeyCredentialCreationOptionsJSON>(isObject),
});

const requestOptions = z.object({
  publicKey: z.custom<PublicKeyCredentialRequestOptionsJSON>(isObject),
});

export async function createPasskeyCredential(options: unknown) {
  const credential = await startRegistration({
    optionsJSON: creationOptions.parse(options).publicKey,
  });
  return jsonObject.parse(credential);
}

export async function assertPasskeyCredential(options: unknown) {
  const credential = await startAuthentication({
    optionsJSON: requestOptions.parse(options).publicKey,
  });
  return jsonObject.parse(credential);
}
