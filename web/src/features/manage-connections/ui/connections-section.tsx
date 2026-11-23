import { useSuspenseQuery } from "@tanstack/react-query";
import { Suspense, useState } from "react";

import { type Connection, connectionsQuery } from "@/entities/connection";
import { meQuery } from "@/entities/user";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/shared/ui/alert-dialog";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Section, SectionEmpty, SectionList, SectionRow } from "@/shared/ui/section";

import { useRevokeConnection } from "../api/use-connection-mutations";
import { EditConnectionDialog } from "./edit-connection-dialog";
import { scopeSummary } from "./scope-summary";

export function ConnectionsSection() {
  const { data: connections } = useSuspenseQuery(connectionsQuery());
  const { data: me } = useSuspenseQuery(meQuery());
  const [editing, setEditing] = useState<Connection | null>(null);
  const [revoking, setRevoking] = useState<Connection | null>(null);

  return (
    <Section title="Connections">
      {connections.length === 0 ? (
        <SectionEmpty>No agents connected.</SectionEmpty>
      ) : (
        <SectionList aria-label="Connections">
          {connections.map((connection) => (
            <SectionRow key={connection.id}>
              <div className="min-w-0">
                <p className="flex items-center gap-2 font-medium">
                  <span className="truncate">
                    {connection.user_id === me.id
                      ? connection.name
                      : `${connection.owner}/${connection.name}`}
                  </span>
                  {connection.can_write ? null : <Badge variant="secondary">Read only</Badge>}
                </p>
                <p className="truncate text-xs text-muted-foreground">
                  {scopeSummary(connection.scopes)},{" "}
                  {connection.last_used_at
                    ? `last used ${formatRelativeTime(connection.last_used_at)}`
                    : "never used"}
                </p>
              </div>
              <div className="flex gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={`Edit ${connection.name}`}
                  onClick={() => setEditing(connection)}
                >
                  Edit
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={`Revoke ${connection.name}`}
                  onClick={() => setRevoking(connection)}
                >
                  Revoke
                </Button>
              </div>
            </SectionRow>
          ))}
        </SectionList>
      )}
      {editing ? (
        <Suspense fallback={null}>
          <EditConnectionDialog connection={editing} onClose={() => setEditing(null)} />
        </Suspense>
      ) : null}
      {revoking ? (
        <RevokeConnectionDialog connection={revoking} onClose={() => setRevoking(null)} />
      ) : null}
    </Section>
  );
}

type RevokeConnectionDialogProps = { connection: Connection; onClose: () => void };

function RevokeConnectionDialog({ connection, onClose }: RevokeConnectionDialogProps) {
  const revoke = useRevokeConnection();
  return (
    <AlertDialog open onOpenChange={(open) => !open && onClose()}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Revoke {connection.name}?</AlertDialogTitle>
          <AlertDialogDescription>
            The agent loses access at once. Connect it again to restore access.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <Button
            variant="destructive"
            disabled={revoke.isPending}
            onClick={() => revoke.mutate(connection.id, { onSettled: onClose })}
          >
            Revoke
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
