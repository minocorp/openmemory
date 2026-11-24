import { ChevronRight } from "lucide-react";

import { CopySnippet } from "@/shared/ui/copy-snippet";

import { agentSetups } from "../model/agent-setups";

export function ConnectAgent() {
  const setups = agentSetups(window.location.origin);
  return (
    <section aria-label="Connect an agent">
      <h2 className="mb-3 text-sm font-medium">Connect an agent</h2>
      <div className="divide-y rounded-lg border">
        {setups.map((setup) => (
          <details key={setup.id} className="group">
            <summary className="flex h-11 cursor-pointer list-none items-center justify-between px-3.5 -outline-offset-2 transition-colors group-first:rounded-t-lg group-last:not-group-open:rounded-b-lg marker:hidden hover:bg-accent/60">
              {setup.label}
              <ChevronRight className="size-4 text-muted-foreground transition-transform group-open:rotate-90" />
            </summary>
            <div className="grid gap-2 px-3.5 pt-1 pb-3.5">
              {setup.steps.length > 0 ? (
                <ol className="list-inside list-decimal text-sm text-muted-foreground">
                  {setup.steps.map((step) => (
                    <li key={step}>{step}</li>
                  ))}
                </ol>
              ) : null}
              {setup.snippets.map((snippet) => (
                <CopySnippet
                  key={snippet.label}
                  label={`${setup.label} ${snippet.label}`}
                  text={snippet.text}
                />
              ))}
            </div>
          </details>
        ))}
      </div>
    </section>
  );
}
