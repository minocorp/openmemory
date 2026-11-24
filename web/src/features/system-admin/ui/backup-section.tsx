import { useSuspenseQuery } from "@tanstack/react-query";
import { TriangleAlert } from "lucide-react";

import { backupWarnings, systemQuery } from "@/entities/system";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { Section, SectionList, SectionRow } from "@/shared/ui/section";

const secondsAgo = (seconds: number | null | undefined) =>
  seconds === null || seconds === undefined
    ? "never"
    : formatRelativeTime(new Date(Date.now() - seconds * 1000).toISOString());

export function BackupSection() {
  const { data: system } = useSuspenseQuery(systemQuery());
  const warnings = backupWarnings(system);
  const { backup } = system;

  return (
    <Section title="Backups">
      {warnings.length > 0 ? (
        <div
          role="alert"
          className="flex gap-2 rounded-lg bg-destructive/8 px-3.5 py-3 text-sm text-destructive"
        >
          <TriangleAlert className="mt-0.5 size-4 shrink-0" />
          <ul className="grid gap-1">
            {warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </div>
      ) : null}
      <SectionList aria-label="Backup status">
        <SectionRow>
          <span>Database</span>
          <span className="text-muted-foreground">{system.database}</span>
        </SectionRow>
        {backup ? (
          <>
            <SectionRow>
              <span>Last backup</span>
              <span className="text-muted-foreground">
                {secondsAgo(backup.last_backup_age_seconds)}
              </span>
            </SectionRow>
            <SectionRow>
              <span>Last full backup</span>
              <span className="text-muted-foreground">
                {secondsAgo(backup.last_full_age_seconds)}
              </span>
            </SectionRow>
            <SectionRow>
              <span>WAL archive</span>
              <span className="text-muted-foreground">
                {secondsAgo(backup.wal_archive_age_seconds)}
              </span>
            </SectionRow>
            <SectionRow>
              <span>File snapshot</span>
              <span className="text-muted-foreground">{secondsAgo(backup.blobs_age_seconds)}</span>
            </SectionRow>
            <SectionRow>
              <span>Restore test</span>
              <span className="text-muted-foreground">
                {backup.restore_test
                  ? `${backup.restore_test.ok ? "passed" : "failed"} ${formatRelativeTime(backup.restore_test.at)}`
                  : "never"}
              </span>
            </SectionRow>
            <SectionRow>
              <span>Off-host copy</span>
              <span className="text-muted-foreground">{backup.off_host ? "yes" : "no"}</span>
            </SectionRow>
          </>
        ) : null}
      </SectionList>
    </Section>
  );
}
