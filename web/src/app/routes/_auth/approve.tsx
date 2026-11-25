import { createFileRoute } from "@tanstack/react-router";

import { ApprovePage, approveSearchSchema } from "@/pages/approve";
import { pageTitle } from "@/shared/lib/page-title";

export const Route = createFileRoute("/_auth/approve")({
  head: () => pageTitle("Approve access"),
  validateSearch: approveSearchSchema,
  component: ApproveRoute,
});

function ApproveRoute() {
  const { consent } = Route.useSearch();
  return <ApprovePage consent={consent} />;
}
