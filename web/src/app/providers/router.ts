import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";
import { createRouter } from "@tanstack/react-router";

import { isApiError } from "@/shared/api";

import { routeTree } from "../routeTree.gen";

const maxRetries = 2;

const publicPaths = ["/login", "/bootstrap", "/register", "/recovery"];

function handleUnauthenticated(error: unknown) {
  if (isApiError(error, 401) && !publicPaths.includes(router.state.location.pathname)) {
    queryClient.clear();
    void router.navigate({ to: "/login" });
  }
}

export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error, query) => {
      if (query.queryKey[0] !== "me") {
        handleUnauthenticated(error);
      }
    },
  }),
  mutationCache: new MutationCache({ onError: handleUnauthenticated }),
  defaultOptions: {
    queries: {
      staleTime: 15_000,
      retry: (failureCount, error) =>
        failureCount < maxRetries && !(isApiError(error) && error.status < 500),
    },
  },
});

export const router = createRouter({
  routeTree,
  context: { queryClient },
  defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
