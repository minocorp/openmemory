import type { QueryClient } from "@tanstack/react-query";
import { createRootRouteWithContext, HeadContent, Outlet } from "@tanstack/react-router";

import { NotFoundPage } from "@/pages/not-found";
import { pageTitle } from "@/shared/lib/page-title";
import { ErrorState } from "@/shared/ui/error-state";

export const Route = createRootRouteWithContext<{ queryClient: QueryClient }>()({
  head: () => pageTitle(),
  component: RootLayout,
  notFoundComponent: NotFoundPage,
  errorComponent: ({ error, reset }) => <ErrorState error={error} onRetry={reset} />,
});

function RootLayout() {
  return (
    <>
      <HeadContent />
      <Outlet />
    </>
  );
}
