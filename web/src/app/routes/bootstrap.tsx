import { createFileRoute } from "@tanstack/react-router";

import { BootstrapPage, bootstrapSearchSchema } from "@/pages/bootstrap";

export const Route = createFileRoute("/bootstrap")({
  validateSearch: bootstrapSearchSchema,
  component: BootstrapRoute,
});

function BootstrapRoute() {
  const { token } = Route.useSearch();
  return <BootstrapPage token={token} />;
}
