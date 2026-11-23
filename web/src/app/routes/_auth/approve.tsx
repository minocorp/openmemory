import { createFileRoute } from "@tanstack/react-router";

import { ApprovePage, approveSearchSchema } from "@/pages/approve";

export const Route = createFileRoute("/_auth/approve")({
  validateSearch: approveSearchSchema,
  component: ApproveRoute,
});

function ApproveRoute() {
  const { consent } = Route.useSearch();
  return <ApprovePage consent={consent} />;
}
