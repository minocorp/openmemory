import { z } from "@/shared/lib/zod";

const problemTypePrefix = "urn:memory:problem:";

const bodySchema = z.object({
  type: z.string(),
  field: z.string().nullish(),
  line: z.number().nullish(),
  reason: z.string().nullish(),
  pattern: z.string().nullish(),
});

export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly field: string | null;
  readonly line: number | null;
  readonly reason: string | null;
  readonly pattern: string | null;

  constructor(status: number, body: unknown) {
    const parsed = bodySchema.safeParse(body);
    const detail = parsed.success ? parsed.data : null;
    const code = detail?.type.replace(problemTypePrefix, "") ?? "unknown_error";
    super(code);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.field = detail?.field ?? null;
    this.line = detail?.line ?? null;
    this.reason = detail?.reason ?? null;
    this.pattern = detail?.pattern ?? null;
  }
}

export function isApiError(error: unknown, status?: number): error is ApiError {
  return error instanceof ApiError && (status === undefined || error.status === status);
}

const quoteMessages: Record<string, string> = {
  not_in_text: "That quote does not occur in the cited file page.",
  no_page: "The cited file has no such page.",
  no_file: "No file exists at the cited path.",
  no_text: "The cited file page has no text to check a quote against. Cite it without a quote.",
};

const messages: Record<string, string> = {
  unauthenticated: "Your session has ended. Sign in again.",
  invalid_credentials: "That passkey was not accepted.",
  invalid_token: "This link is invalid or has expired.",
  name_taken: "That name is already taken.",
  passkey_taken: "That passkey is already registered.",
  passkey_rejected: "The passkey could not be verified.",
  last_passkey: "You cannot remove your only passkey.",
  last_admin: "There must be at least one admin.",
  forbidden: "Only an admin can do that.",
  path_taken: "Something already exists at that path.",
  reserved_path: "That path is reserved.",
  same_path: "That is already the page's path.",
  not_found: "Not found.",
  read_only: "This connection cannot write.",
  too_large: "The page is too large.",
  future_date: "That date is after today.",
  timeline_changed: "Existing timeline lines cannot be changed.",
  no_change: "Nothing changed.",
  already_current: "That version is already current.",
  precondition_failed: "The page changed since you opened it.",
  purged: "This page was purged.",
  rate_limited: "Too many attempts. Wait a moment and try again.",
};

export function describeError(error: unknown): string {
  if (error instanceof ApiError) {
    return describeApiError(error);
  }
  if (error instanceof DOMException && error.name === "NotAllowedError") {
    return "The passkey prompt was cancelled or timed out.";
  }
  if (error instanceof Error) {
    return error.message;
  }
  return "Something went wrong.";
}

function describeApiError(error: ApiError): string {
  if (error.code === "grammar") {
    return `Line ${error.line ?? "?"}: ${error.reason?.replaceAll("_", " ") ?? "invalid page"}.`;
  }
  if (error.code === "credential") {
    return `The text looks like a secret (${error.pattern ?? "credential"}) and was rejected.`;
  }
  if (error.code === "quote_unverified") {
    return quoteMessages[error.reason ?? ""] ?? "The quote could not be checked against the file.";
  }
  if (error.status === 429) {
    return messages["rate_limited"] ?? "Too many attempts.";
  }
  return messages[error.code] ?? `Request failed (${error.code.replaceAll("_", " ")}).`;
}
