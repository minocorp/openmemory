import type { BackupProblem, SystemStatus } from "./types";

const problemText: Record<BackupProblem, string> = {
  status_missing: "The backup service has not reported a status.",
  status_stale: "The backup service stopped reporting.",
  full_backup_stale: "The last full backup is too old.",
  backup_stale: "The last backup is too old.",
  wal_archive_stale: "The WAL archive is behind.",
  blob_backup_stale: "The file snapshot is too old.",
  restore_test_failed: "The last restore test failed.",
  restore_test_stale: "The restore test is overdue.",
};

export function backupWarnings(system: SystemStatus): string[] {
  const { backup } = system;
  if (backup === null || backup === undefined) {
    return ["No backup is configured."];
  }
  const warnings = backup.problems.map((problem) => problemText[problem]);
  return backup.off_host
    ? warnings
    : ["Backups stay on this host. Set an off-host S3 target.", ...warnings];
}
