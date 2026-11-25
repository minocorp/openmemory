import { createFileRoute } from "@tanstack/react-router";

import { BootstrapPage, bootstrapSearchSchema } from "@/pages/bootstrap";
import { pageTitle } from "@/shared/lib/page-title";

export const Route = createFileRoute("/bootstrap")({
  head: () => pageTitle("Set up"),
  validateSearch: bootstrapSearchSchema,
  component: BootstrapRoute,
});

function BootstrapRoute() {
  const { token } = Route.useSearch();
  return <BootstrapPage token={token} />;
}
