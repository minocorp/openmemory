import { Download, Upload } from "lucide-react";
import { useRef } from "react";

import { Button } from "@/shared/ui/button";
import { Section, SectionList, SectionRow } from "@/shared/ui/section";

import { useExport, useImport } from "../api/use-transfer";

export function TransferSection() {
  const exportArchive = useExport();
  const importArchive = useImport();
  const fileInput = useRef<HTMLInputElement>(null);
  const report = importArchive.data;

  return (
    <Section title="Export and import">
      <SectionList aria-label="Export and import">
        <SectionRow>
          <span>Export pages and files as a zip</span>
          <Button
            variant="outline"
            size="sm"
            disabled={exportArchive.isPending}
            onClick={() => exportArchive.mutate()}
          >
            <Download /> Export
          </Button>
        </SectionRow>
        <SectionRow>
          <span>Import a zip from an export</span>
          <Button
            variant="outline"
            size="sm"
            disabled={importArchive.isPending}
            onClick={() => fileInput.current?.click()}
          >
            <Upload /> Import
          </Button>
          <input
            ref={fileInput}
            type="file"
            accept=".zip,application/zip"
            aria-label="Import zip"
            className="sr-only"
            onChange={(event) => {
              const file = event.target.files?.[0];
              if (file) {
                importArchive.mutate(file);
              }
              event.target.value = "";
            }}
          />
        </SectionRow>
      </SectionList>
      {report ? (
        <output className="grid gap-1 rounded-lg border px-3.5 py-3 text-sm">
          <p>
            Imported {report.pages} pages and {report.files} files.
            {report.skipped.length > 0 ? ` Skipped ${report.skipped.length}.` : ""}
          </p>
          {report.skipped.length > 0 ? (
            <ul className="grid gap-0.5 text-xs text-muted-foreground">
              {report.skipped.map((entry) => (
                <li key={entry.name}>
                  {entry.name}: {entry.reason.replaceAll("_", " ")}
                </li>
              ))}
            </ul>
          ) : null}
        </output>
      ) : null}
    </Section>
  );
}
