import { useSuspenseQuery } from "@tanstack/react-query";
import { useState } from "react";

import { meQuery, PersonName, type User, usersQuery } from "@/entities/user";
import { Button } from "@/shared/ui/button";
import { Section, SectionList, SectionRow } from "@/shared/ui/section";
import { Switch } from "@/shared/ui/switch";

import { useSetAdmin } from "../api/use-people-mutations";
import { EditUserDialog } from "./edit-user-dialog";

export function UsersSection() {
  const { data: users } = useSuspenseQuery(usersQuery());
  const { data: me } = useSuspenseQuery(meQuery());
  const setAdmin = useSetAdmin();
  const [editing, setEditing] = useState<User | null>(null);

  return (
    <Section title="People">
      <SectionList aria-label="People">
        {users.map((user) => (
          <SectionRow key={user.id}>
            <p className="font-medium">
              <PersonName displayName={user.display_name} username={user.username} />
              {user.id === me.id ? (
                <span className="ml-2 text-xs text-muted-foreground">you</span>
              ) : null}
            </p>
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Button
                variant="ghost"
                size="sm"
                aria-label={`Edit ${user.username}`}
                onClick={() => setEditing(user)}
              >
                Edit
              </Button>
              <span>Admin</span>
              <Switch
                aria-label={`Admin ${user.username}`}
                checked={user.is_admin}
                disabled={setAdmin.isPending}
                onCheckedChange={(isAdmin) => setAdmin.mutate({ id: user.id, isAdmin })}
              />
            </div>
          </SectionRow>
        ))}
      </SectionList>
      {editing ? <EditUserDialog user={editing} onClose={() => setEditing(null)} /> : null}
    </Section>
  );
}
