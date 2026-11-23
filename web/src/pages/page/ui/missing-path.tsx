import { useState } from "react";

import { CreatePageDialog } from "@/features/manage-page";
import { Button } from "@/shared/ui/button";

type MissingPathProps = { path: string };

export function MissingPath({ path }: MissingPathProps) {
  const [isCreating, setIsCreating] = useState(false);
  return (
    <div className="flex h-full items-center justify-center p-8">
      <div className="max-w-sm text-center">
        <h1 className="text-lg font-semibold">Nothing here yet</h1>
        <p className="mt-2 text-sm break-all text-muted-foreground">{path}</p>
        <Button className="mt-5" onClick={() => setIsCreating(true)}>
          Create page here
        </Button>
      </div>
      {isCreating ? (
        <CreatePageDialog initialPath={path} onClose={() => setIsCreating(false)} />
      ) : null}
    </div>
  );
}
