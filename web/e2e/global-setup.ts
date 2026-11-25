import { type ChildProcess, execFileSync, spawn } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { resolve } from "node:path";

import { bootstrapAdmin } from "./support/admin-session";
import {
  apiPort,
  blobDirectory,
  databaseName,
  databaseUrl,
  repoRoot,
  screenshotDirectory,
  webOrigin,
} from "./support/config";

const serverStartTimeoutMs = 60_000;
const bootstrapPattern = /bootstrap\?token=([A-Za-z0-9_-]+)/;

function cargoEnvironment(): NodeJS.ProcessEnv {
  const environment: NodeJS.ProcessEnv = { ...process.env, SQLX_OFFLINE: "true" };
  delete environment["RUSTUP_TOOLCHAIN"];
  return environment;
}

function psql(sql: string) {
  execFileSync(
    "docker",
    [
      "compose",
      "-f",
      resolve(repoRoot, "compose.dev.yml"),
      "exec",
      "-T",
      "postgres",
      "psql",
      "-U",
      "openmemory",
      "-d",
      "postgres",
      "-v",
      "ON_ERROR_STOP=1",
      "-c",
      sql,
    ],
    { stdio: "ignore" },
  );
}

function resetDatabase() {
  psql(`DROP DATABASE IF EXISTS ${databaseName} WITH (FORCE)`);
  psql(`CREATE DATABASE ${databaseName}`);
}

function buildServer() {
  execFileSync("cargo", ["build", "-p", "openmemory-server", "--locked"], {
    cwd: repoRoot,
    env: cargoEnvironment(),
    stdio: "inherit",
  });
}

function startServer(): ChildProcess {
  rmSync(blobDirectory, { recursive: true, force: true });
  return spawn(resolve(repoRoot, "target/debug/openmemory-server"), ["serve"], {
    cwd: repoRoot,
    env: {
      ...process.env,
      NO_COLOR: "1",
      DATABASE_URL: databaseUrl,
      OPENMEMORY_LISTEN: `127.0.0.1:${apiPort}`,
      OPENMEMORY_PUBLIC_URL: webOrigin,
      OPENMEMORY_BLOB_DIR: blobDirectory,
      OPENMEMORY_TRUSTED_PROXIES: "127.0.0.1,::1",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function waitForBootstrapToken(server: ChildProcess): Promise<string> {
  return new Promise((resolveToken, reject) => {
    const timer = setTimeout(
      () => reject(new Error("The server did not print a bootstrap link")),
      serverStartTimeoutMs,
    );
    const onOutput = (chunk: Buffer) => {
      const token = bootstrapPattern.exec(chunk.toString())?.[1];
      if (token) {
        clearTimeout(timer);
        resolveToken(token);
      }
    };
    server.stdout?.on("data", onOutput);
    server.stderr?.on("data", onOutput);
    server.once("exit", (code) => reject(new Error(`The server exited with code ${code}`)));
  });
}

// Playwright loads global setup by its default export
// oxlint-disable-next-line import/no-default-export -- required by Playwright's globalSetup contract
export default async function globalSetup() {
  mkdirSync(screenshotDirectory, { recursive: true });
  resetDatabase();
  buildServer();
  const server = startServer();
  await bootstrapAdmin(await waitForBootstrapToken(server));
  return () => {
    server.kill("SIGTERM");
  };
}
