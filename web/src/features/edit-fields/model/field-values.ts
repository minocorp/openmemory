import type { Page } from "@/entities/page";

export function valuesOf(page: Page, key: string): string[] | null {
  return page.fields.find((field) => field.key === key)?.values ?? null;
}

export function sameValues(a: readonly string[] | null, b: readonly string[] | null): boolean {
  if (a === null || b === null) {
    return a === b;
  }
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

export function parseFieldInput(text: string, wasList: boolean): string[] {
  const trimmed = text.trim();
  if (trimmed === "") {
    return [];
  }
  if (!wasList) {
    return [trimmed];
  }
  return trimmed
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part !== "");
}

const keyPattern = /^[A-Za-z][A-Za-z0-9_-]*$/;

export function isValidFieldKey(key: string): boolean {
  return keyPattern.test(key);
}
