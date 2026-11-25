import { createFileRoute } from "@tanstack/react-router";

import { PagesTab } from "@/pages/pages-tab";
import { pageTitle } from "@/shared/lib/page-title";

export const Route = createFileRoute("/_app/pages")({
  head: () => pageTitle("Pages"),
  component: PagesTab,
});
