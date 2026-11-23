import { ChevronRight } from "lucide-react";

import { agentSetups } from "../model/agent-setups";
import { CopySnippet } from "./copy-snippet";

export function ConnectAgent() {
  const setups = agentSetups(window.location.origin);
  return (
    <section aria-label="Connect an agent">
      <h2 className="mb-2 text-[13px] font-medium">Connect an agent</h2>
      <div className="divide-y rounded-lg border bg-background">
        {setups.map((setup) => (
          <details key={setup.id} className="group px-3 py-2">
            <summary className="flex cursor-pointer list-none items-center justify-between text-[13px] marker:hidden">
              {setup.label}
              <ChevronRight className="size-4 text-muted-foreground transition-transform group-open:rotate-90" />
            </summary>
            <div className="mt-2 space-y-2 pb-1">
              {setup.steps.length > 0 ? (
                <ol className="list-inside list-decimal text-xs text-muted-foreground">
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
