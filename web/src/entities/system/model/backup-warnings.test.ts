import { describe, expect, it } from "vitest";

import { backupWarnings } from "./backup-warnings";

describe("backupWarnings", () => {
  it("warns when no backup is configured", () => {
    expect(backupWarnings({ database: "ok", backup: null })).toEqual(["No backup is configured."]);
  });

  it("warns about a missing off-host target first", () => {
    const warnings = backupWarnings({
      database: "ok",
      backup: { problems: ["backup_stale"], off_host: false },
    });
    expect(warnings).toEqual([
      "Backups stay on this host. Set an off-host S3 target.",
      "The last backup is too old.",
    ]);
  });

  it("is empty when everything is healthy", () => {
    expect(backupWarnings({ database: "ok", backup: { problems: [], off_host: true } })).toEqual(
      [],
    );
  });
});
