import { TriangleAlert } from "lucide-react";

import type { Consent } from "@/entities/consent";
import { Badge } from "@/shared/ui/badge";

type ConsentOriginProps = { consent: Consent };

export function ConsentOrigin({ consent }: ConsentOriginProps) {
  const host = consent.redirect_host;
  return (
    <div className="-mt-3 mb-6 grid gap-2">
      <p className="flex flex-wrap items-center gap-2">
        <span className="text-base font-semibold break-all">{host ?? "Device"}</span>
        {consent.localhost_only ? <Badge variant="outline">This computer only</Badge> : null}
        {consent.client.official ? <Badge variant="secondary">Official memory CLI</Badge> : null}
        {consent.client.unverified ? <Badge variant="destructive">Unverified client</Badge> : null}
      </p>
      {consent.name_matches_redirect === false ? (
        <p role="alert" className="flex items-start gap-2 text-sm text-destructive">
          <TriangleAlert className="mt-0.5 size-4 shrink-0" />
          {consent.client.name} does not match {host}
        </p>
      ) : null}
    </div>
  );
}
