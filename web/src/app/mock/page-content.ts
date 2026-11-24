import type { Schemas } from "./store";

type Parsed = Pick<Schemas["PageView"], "fields" | "body" | "timeline">;

const fence = "---";
const timelineHeading = "## Timeline";
const timelinePattern = /^(\d{4}-\d{2}-\d{2}) \| (.+?) — (.+)$/;
const fileSourcePattern = /^(\S+\.\w+)(?:#p(\d+))?(?: "(.+)")?$/;

export function parsePage(content: string): Parsed {
  const lines = content.split(/(?<=\n)/);
  const close = lines[0]?.trimEnd() === fence ? closingFence(lines) : -1;
  const headEnd = close === -1 ? 0 : close + 1;
  const tailStart = timelineStart(lines, headEnd);
  return {
    fields: lines.slice(1, close === -1 ? 0 : close).flatMap(parseField),
    body: lines.slice(headEnd, tailStart).join(""),
    timeline: lines.slice(tailStart + 1).flatMap(parseTimelineLine),
  };
}

export function appendTimelineLine(content: string, line: string): string {
  const parsed = parsePage(content);
  const hasHeading = parsed.timeline.length > 0 || content.includes(`\n${timelineHeading}\n`);
  const base = content.endsWith("\n") || content === "" ? content : `${content}\n`;
  return hasHeading ? `${base}${line}\n` : `${base}\n${timelineHeading}\n${line}\n`;
}

function closingFence(lines: string[]): number {
  return lines.findIndex((line, index) => index > 0 && line.trimEnd() === fence);
}

function timelineStart(lines: string[], from: number): number {
  const index = lines.findLastIndex(
    (line, position) => position >= from && line.trimEnd() === timelineHeading,
  );
  return index === -1 ? lines.length : index;
}

function parseField(line: string): Schemas["FieldView"][] {
  const separator = line.indexOf(":");
  if (separator <= 0) {
    return [];
  }
  const key = line.slice(0, separator).trim();
  const raw = line.slice(separator + 1).trim();
  const values = raw.startsWith("[") && raw.endsWith("]") ? raw.slice(1, -1).split(",") : [raw];
  return [{ key, values: values.map(unquote).filter((value) => value !== "") }];
}

function unquote(value: string): string {
  const trimmed = value.trim();
  return trimmed.startsWith('"') && trimmed.endsWith('"') ? trimmed.slice(1, -1) : trimmed;
}

function parseTimelineLine(line: string): Schemas["TimelineView"][] {
  const match = timelinePattern.exec(line.trimEnd());
  if (!match?.[1] || !match[2] || !match[3]) {
    return [];
  }
  const file = fileSourcePattern.exec(match[2]);
  return [
    {
      date: match[1],
      source: match[2],
      text: match[3],
      file: file?.[1]
        ? { path: file[1], page: file[2] ? Number(file[2]) : null, quote: file[3] ?? null }
        : null,
    },
  ];
}
