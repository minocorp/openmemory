import { createFileRoute } from "@tanstack/react-router";

import { RegisterPage, registerSearchSchema } from "@/pages/register";
import { pageTitle } from "@/shared/lib/page-title";

export const Route = createFileRoute("/register")({
  head: () => pageTitle("Register"),
  validateSearch: registerSearchSchema,
  component: RegisterRoute,
});

function RegisterRoute() {
  const { token } = Route.useSearch();
  return <RegisterPage token={token} />;
}
