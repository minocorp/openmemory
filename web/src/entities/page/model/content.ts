import type { Page } from "./types";

const fence = "---";

type Lines = string[];

export function splitContent(page: Pick<Page, "content" | "body">): {
  head: string;
  tail: string;
} {
  const length = headLength(page.content);
  return {
    head: page.content.slice(0, length),
    tail: page.content.slice(length + page.body.length),
  };
}

export function withBody(page: Pick<Page, "content" | "body">, body: string): string {
  const { head, tail } = splitContent(page);
  const separator = tail !== "" && !body.endsWith("\n") ? "\n" : "";
  return `${head}${body}${separator}${tail}`;
}

export function withHead(page: Pick<Page, "content" | "body">, head: string): string {
  const { tail } = splitContent(page);
  return `${head}${page.body}${tail}`;
}

export function setField(head: string, key: string, values: readonly string[]): string {
  const line = fieldLine(key, values);
  const lines = splitLines(head);
  if (lines.length === 0) {
    return `${fence}\n${line}${fence}\n`;
  }
  const close = closingFence(lines);
  const range = fieldRange(lines, key, close);
  if (range) {
    return [...lines.slice(0, range.start), line, ...lines.slice(range.end)].join("");
  }
  return [...lines.slice(0, close), line, ...lines.slice(close)].join("");
}

export function removeField(head: string, key: string): string {
  const lines = splitLines(head);
  if (lines.length === 0) {
    return head;
  }
  const range = fieldRange(lines, key, closingFence(lines));
  if (!range) {
    return head;
  }
  return [...lines.slice(0, range.start), ...lines.slice(range.end)].join("");
}

function headLength(content: string): number {
  const lines = splitLines(content);
  if (lines[0]?.trimEnd() !== fence) {
    return 0;
  }
  const close = lines.findIndex((line, index) => index > 0 && line.trimEnd() === fence);
  return close === -1 ? 0 : lines.slice(0, close + 1).join("").length;
}

function splitLines(text: string): Lines {
  return text === "" ? [] : text.split(/(?<=\n)/);
}

function closingFence(lines: Lines): number {
  return lines.findIndex((line, index) => index > 0 && line.trimEnd() === fence);
}

function fieldRange(
  lines: Lines,
  key: string,
  close: number,
): { start: number; end: number } | null {
  const start = lines.findIndex(
    (line, index) => index > 0 && index < close && line.startsWith(`${key}:`),
  );
  if (start === -1) {
    return null;
  }
  let end = start + 1;
  while (end < close && isContinuation(lines[end] ?? "")) {
    end += 1;
  }
  return { start, end };
}

function isContinuation(line: string): boolean {
  return /^\s+\S/.test(line) || line.startsWith("- ");
}

function fieldLine(key: string, values: readonly string[]): string {
  const [only, ...rest] = values;
  if (only !== undefined && rest.length === 0) {
    return `${key}: ${scalar(only)}\n`;
  }
  return `${key}: [${values.map(scalar).join(", ")}]\n`;
}

function scalar(value: string): string {
  const needsQuotes = /^\s|\s$|^[-?:,[\]{}#&*!|>'"%@`]|: | #|^$/.test(value);
  return needsQuotes ? JSON.stringify(value) : value;
}
