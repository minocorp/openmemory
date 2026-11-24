import { useState } from "react";

import { ConnectAgent } from "@/features/connect-agent";
import { CreatePageDialog } from "@/features/manage-page";
import { Button } from "@/shared/ui/button";

export function FirstRunPanel() {
  const [isCreating, setIsCreating] = useState(false);
  return (
    <section aria-label="First run" className="grid gap-10">
      <div className="flex items-center justify-between gap-3 rounded-lg border py-2.5 pr-2.5 pl-3.5">
        <span className="text-sm">No start page</span>
        <Button size="sm" onClick={() => setIsCreating(true)}>
          Create start page
        </Button>
      </div>
      <ConnectAgent />
      {isCreating ? (
        <CreatePageDialog initialPath="start" onClose={() => setIsCreating(false)} />
      ) : null}
    </section>
  );
}
