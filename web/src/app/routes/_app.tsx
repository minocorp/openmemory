import { createFileRoute } from "@tanstack/react-router";

import { requireSession } from "../model/require-session";
import { AppLayout } from "../ui/app-layout";

export const Route = createFileRoute("/_app")({
  beforeLoad: ({ context, location }) => requireSession(context.queryClient, location),
  component: AppLayout,
});
