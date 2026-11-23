import { createFileRoute } from "@tanstack/react-router";

import { SettingsPage, settingsSearchSchema } from "@/pages/settings";

export const Route = createFileRoute("/_app/settings")({
  validateSearch: settingsSearchSchema,
  component: SettingsRoute,
});

function SettingsRoute() {
  const { tab } = Route.useSearch();
  return <SettingsPage tab={tab} />;
}
