import { createFileRoute } from "@tanstack/react-router";

import { RegisterPage, registerSearchSchema } from "@/pages/register";

export const Route = createFileRoute("/register")({
  validateSearch: registerSearchSchema,
  component: RegisterRoute,
});

function RegisterRoute() {
  const { token } = Route.useSearch();
  return <RegisterPage token={token} />;
}
