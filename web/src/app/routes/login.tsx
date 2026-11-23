import { createFileRoute } from "@tanstack/react-router";

import { LoginPage, loginSearchSchema } from "@/pages/login";

export const Route = createFileRoute("/login")({
  validateSearch: loginSearchSchema,
  component: LoginRoute,
});

function LoginRoute() {
  const { next } = Route.useSearch();
  return <LoginPage next={next} />;
}
