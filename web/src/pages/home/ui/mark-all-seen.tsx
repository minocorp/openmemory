import { useMarkSeen } from "@/features/mark-seen";
import { Button } from "@/shared/ui/button";

export function MarkAllSeen({ seq }: { seq: number }) {
  const markSeen = useMarkSeen();
  return (
    <Button
      variant="ghost"
      size="sm"
      className="-mr-2.5"
      disabled={markSeen.isPending}
      onClick={() => markSeen.mutate(seq)}
    >
      Mark all seen
    </Button>
  );
}
