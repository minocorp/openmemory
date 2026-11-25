import { Link } from "@tanstack/react-router";

import { ConsentCard } from "./consent-card";

type ConsentNoticeProps = { title: string; detail?: string };

export function ConsentNotice({ title, detail }: ConsentNoticeProps) {
  return (
    <ConsentCard title={title}>
      {detail ? <p className="text-sm text-muted-foreground">{detail}</p> : null}
      <Link to="/" className="mt-6 inline-block text-sm underline underline-offset-4">
        Back to OpenMemory
      </Link>
    </ConsentCard>
  );
}
