import { useSuspenseQuery } from "@tanstack/react-query";
import { Copy, Plus } from "lucide-react";
import { useState } from "react";

import { inviteLink, invitesQuery } from "@/entities/invite";
import { PersonName } from "@/entities/user";
import { formatRelativeTime } from "@/shared/lib/relative-time";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Section, SectionEmpty, SectionList, SectionRow } from "@/shared/ui/section";
import { toast } from "@/shared/ui/toaster";

import { useCreateInvite, useRevokeInvite } from "../api/use-people-mutations";

export function InvitesSection() {
  const { data: invites } = useSuspenseQuery(invitesQuery());
  const create = useCreateInvite();
  const revoke = useRevokeInvite();
  const [link, setLink] = useState<string | null>(null);

  const handleCreate = () =>
    create.mutate(undefined, {
      onSuccess: ({ token }) => setLink(inviteLink(globalThis.location.origin, token)),
    });

  const handleCopy = async (value: string) => {
    await navigator.clipboard.writeText(value);
    toast.success("Link copied");
  };

  return (
    <Section
      title="Invites"
      action={
        <Button variant="outline" size="sm" disabled={create.isPending} onClick={handleCreate}>
          <Plus /> New invite
        </Button>
      }
    >
      {link ? (
        <div className="flex gap-2">
          <Input
            readOnly
            aria-label="Invite link"
            value={link}
            onFocus={(e) => e.target.select()}
          />
          <Button variant="outline" onClick={() => void handleCopy(link)}>
            <Copy /> Copy
          </Button>
        </div>
      ) : null}
      {invites.length === 0 ? (
        <SectionEmpty>No open invites.</SectionEmpty>
      ) : (
        <SectionList aria-label="Open invites">
          {invites.map((invite) => (
            <SectionRow key={invite.id}>
              <div className="min-w-0">
                <p className="font-medium">
                  Invited by{" "}
                  <PersonName displayName={invite.inviter_display_name} username={invite.inviter} />
                </p>
                <p className="text-xs text-muted-foreground">
                  Expires {formatRelativeTime(invite.expires_at)}
                </p>
              </div>
              <Button
                variant="ghost"
                size="sm"
                aria-label={`Revoke invite from ${invite.inviter}, ${invite.id}`}
                disabled={revoke.isPending}
                onClick={() => revoke.mutate(invite.id)}
              >
                Revoke
              </Button>
            </SectionRow>
          ))}
        </SectionList>
      )}
    </Section>
  );
}
