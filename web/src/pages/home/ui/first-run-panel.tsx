import { useState } from "react";

import { ConnectAgent } from "@/features/connect-agent";
import { CreatePageDialog } from "@/features/manage-page";
import { Button } from "@/shared/ui/button";

export function FirstRunPanel() {
  const [isCreating, setIsCreating] = useState(false);
  return (
    <section aria-label="First run" className="space-y-6">
      <div className="flex items-center justify-between gap-3 rounded-lg border bg-background px-3 py-2.5">
        <span className="text-[13px]">No start page</span>
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
