import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import { consentQuery } from "@/entities/consent";
import { ConsentForm, type Outcome } from "@/features/approve-connection";
import { describeError, isApiError } from "@/shared/api";
import { Skeleton } from "@/shared/ui/skeleton";

import { ConsentCard } from "./consent-card";
import { ConsentNotice } from "./consent-notice";
import { ConsentOrigin } from "./consent-origin";

type ConsentScreenProps = { reference: string };

type Decided = "approved" | "denied" | null;

export function ConsentScreen({ reference }: ConsentScreenProps) {
  const consent = useQuery(consentQuery(reference));
  const [decided, setDecided] = useState<Decided>(null);

  const handleDecided = (outcome: Outcome, decision: Exclude<Decided, null>) => {
    if (outcome.kind === "redirect") {
      window.location.assign(outcome.target);
      return;
    }
    setDecided(decision);
  };

  if (decided === "approved") {
    return <ConsentNotice title="Connected" detail="You can close this page." />;
  }
  if (decided === "denied") {
    return <ConsentNotice title="Denied" />;
  }
  if (consent.isPending) {
    return (
      <ConsentCard title="Connect">
        <Skeleton className="h-40" />
      </ConsentCard>
    );
  }
  if (consent.isError) {
    return isApiError(consent.error, 404) ? (
      <ConsentNotice
        title="This request has expired"
        detail="Start the connection again from the app."
      />
    ) : (
      <ConsentNotice title="Could not load the request" detail={describeError(consent.error)} />
    );
  }
  return (
    <ConsentCard title={`Connect ${consent.data.client.name}`}>
      <ConsentOrigin consent={consent.data} />
      <ConsentForm reference={reference} consent={consent.data} onDecided={handleDecided} />
    </ConsentCard>
  );
}
