import { createFileRoute, Outlet } from "@tanstack/react-router";

import { requireSession } from "../model/require-session";

export const Route = createFileRoute("/_auth")({
  beforeLoad: ({ context, location }) => requireSession(context.queryClient, location),
  component: Outlet,
});
