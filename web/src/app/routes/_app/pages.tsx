import { createFileRoute } from "@tanstack/react-router";

import { PagesTab } from "@/pages/pages-tab";

export const Route = createFileRoute("/_app/pages")({ component: PagesTab });
