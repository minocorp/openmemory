import { createFileRoute } from "@tanstack/react-router";

import { RecoveryPage } from "@/pages/recovery";
import { pageTitle } from "@/shared/lib/page-title";

export const Route = createFileRoute("/recovery")({
  head: () => pageTitle("Recovery"),
  component: RecoveryPage,
});
