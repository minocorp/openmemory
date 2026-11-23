import { type FormEvent, useState } from "react";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";

import { normalizeUserCode } from "../model/search";
import { ConsentCard } from "./consent-card";
import { ConsentScreen } from "./consent-screen";

type DevicePageProps = {
  userCode: string | undefined;
  onCodeEntered: (code: string) => void;
};

export function DevicePage({ userCode, onCodeEntered }: DevicePageProps) {
  const [typed, setTyped] = useState("");

  if (userCode) {
    return <ConsentScreen key={userCode} reference={userCode} />;
  }

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    onCodeEntered(normalizeUserCode(typed));
  };

  return (
    <ConsentCard title="Connect a device">
      <form className="grid gap-4" onSubmit={handleSubmit}>
        <div className="grid gap-2">
          <Label htmlFor="user-code">Code</Label>
          <Input
            id="user-code"
            value={typed}
            onChange={(event) => setTyped(event.target.value)}
            autoComplete="off"
            autoCapitalize="characters"
            className="font-mono uppercase"
            required
          />
        </div>
        <Button type="submit" disabled={typed.trim() === ""}>
          Continue
        </Button>
      </form>
    </ConsentCard>
  );
}
