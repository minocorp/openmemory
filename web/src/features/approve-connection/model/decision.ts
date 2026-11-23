import type { components } from "@/shared/api";

type Decision = components["schemas"]["Decision"];

export type Outcome = { kind: "redirect"; target: string } | { kind: "done" };

export function outcomeOf(decision: Decision): Outcome {
  return decision.redirect_uri
    ? { kind: "redirect", target: decision.redirect_uri }
    : { kind: "done" };
}

export const hasScopeSelection = (scopes: string[]) => scopes.length > 0;

const maxNameLength = 64;

export type NameProblem = "empty" | "long" | "slash";

export function connectionNameProblem(raw: string): NameProblem | null {
  const name = raw.trim();
  if (name === "") {
    return "empty";
  }
  if (Array.from(name).length > maxNameLength) {
    return "long";
  }
  return name.includes("/") ? "slash" : null;
}
