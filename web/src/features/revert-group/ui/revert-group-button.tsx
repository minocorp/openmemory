import { RevertBlockedError } from "@/entities/change";
import { Button } from "@/shared/ui/button";

import { useRevertGroup } from "../api/use-revert-group";

const reasons: Record<string, string> = {
  changed_since: "changed after this group",
  path_taken: "its path is taken",
};

type RevertGroupButtonProps = { groupId: number };

export function RevertGroupButton({ groupId }: RevertGroupButtonProps) {
  const revert = useRevertGroup();
  const blocked = revert.error instanceof RevertBlockedError ? revert.error.blocked : [];
  if (revert.isSuccess) {
    return <p className="text-right text-xs text-muted-foreground">Reverted</p>;
  }
  return (
    <div className="flex flex-col items-end gap-2">
      <Button
        variant="outline"
        size="sm"
        disabled={revert.isPending}
        onClick={() => revert.mutate(groupId)}
      >
        Revert group
      </Button>
      {blocked.length > 0 ? (
        <div
          role="alert"
          className="w-full rounded-md bg-destructive/10 px-3 py-2 text-left text-xs"
        >
          <p className="font-medium text-destructive">Nothing was reverted. Blocked by:</p>
          <ul className="mt-1 space-y-0.5">
            {blocked.map((page) => (
              <li key={page.path}>
                <span className="font-mono">{page.path}</span>{" "}
                <span className="text-muted-foreground">{reasons[page.reason] ?? page.reason}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
