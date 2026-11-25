import { resolve } from "node:path";

const instance = Number(process.env["E2E_INSTANCE"] ?? "0");

export const webPort = 4173 + instance;
export const apiPort = 8081 + instance;
export const embedded = process.env["E2E_EMBEDDED"] === "1";
export const webOrigin = `http://localhost:${embedded ? apiPort : webPort}`;
export const apiOrigin = `http://127.0.0.1:${apiPort}`;
export const databaseName = instance === 0 ? "openmemory_e2e" : `openmemory_e2e_${instance}`;
export const databaseUrl = `postgres://openmemory:openmemory@127.0.0.1:5441/${databaseName}`;
export const repoRoot = resolve(import.meta.dirname, "../../..");
export const runDirectory = resolve(repoRoot, "run");
export const blobDirectory = resolve(runDirectory, `e2e-blobs-${instance}`);
export const screenshotDirectory = resolve(runDirectory, "ui");
