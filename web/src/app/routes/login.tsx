import { createFileRoute } from "@tanstack/react-router";

import { LoginPage, loginSearchSchema } from "@/pages/login";
import { pageTitle } from "@/shared/lib/page-title";

export const Route = createFileRoute("/login")({
  head: () => pageTitle("Sign in"),
  validateSearch: loginSearchSchema,
  component: LoginRoute,
});

function LoginRoute() {
  const { next } = Route.useSearch();
  return <LoginPage next={next} />;
}
