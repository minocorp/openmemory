import { createFileRoute } from "@tanstack/react-router";

import { SettingsPage, settingsSearchSchema } from "@/pages/settings";
import { pageTitle } from "@/shared/lib/page-title";

export const Route = createFileRoute("/_app/settings")({
  head: () => pageTitle("Settings"),
  validateSearch: settingsSearchSchema,
  component: SettingsRoute,
});

function SettingsRoute() {
  const { tab } = Route.useSearch();
  return <SettingsPage tab={tab} />;
}
