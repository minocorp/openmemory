import { useSuspenseQuery } from "@tanstack/react-query";
import { useState, type FormEvent } from "react";

import { meQuery } from "@/entities/user";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Section } from "@/shared/ui/section";

import { useUpdateMyProfile } from "../api/use-profile-mutations";

export function ProfileSection() {
  const { data: me } = useSuspenseQuery(meQuery());
  const update = useUpdateMyProfile();
  const [username, setUsername] = useState(me.username);
  const [displayName, setDisplayName] = useState(me.display_name);
  const unchanged = username === me.username && displayName === me.display_name;

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    update.mutate({ username, displayName });
  };

  return (
    <Section title="Profile">
      <form className="grid gap-3" onSubmit={handleSubmit}>
        <div className="grid gap-2">
          <Label htmlFor="profile-username">Username</Label>
          <Input
            id="profile-username"
            value={username}
            autoComplete="off"
            pattern="[A-Za-z0-9_.\-]{1,32}"
            maxLength={32}
            required
            onChange={(event) => setUsername(event.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            Signs you in and prefixes your connections, as in {username || "username"}/claude-code.
          </p>
        </div>
        <div className="grid gap-2">
          <Label htmlFor="profile-display-name">Display name</Label>
          <Input
            id="profile-display-name"
            value={displayName}
            autoComplete="off"
            maxLength={64}
            required
            onChange={(event) => setDisplayName(event.target.value)}
          />
        </div>
        <div>
          <Button type="submit" variant="outline" disabled={unchanged || update.isPending}>
            Save profile
          </Button>
        </div>
      </form>
    </Section>
  );
}
