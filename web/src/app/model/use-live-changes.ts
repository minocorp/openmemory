import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";

import { followChanges } from "@/entities/change";
import { isApiError } from "@/shared/api";

import { createLiveCache } from "./live-cache";

export function useLiveChanges() {
  const queryClient = useQueryClient();
  useEffect(() => {
    const controller = new AbortController();
    const cache = createLiveCache(queryClient);
    const follow = async () => {
      try {
        await followChanges(cache, controller.signal);
      } catch (error) {
        if (!isApiError(error, 401)) {
          throw error;
        }
        // Refetching active queries hits the same 401 and sends the user to sign in.
        await queryClient.invalidateQueries();
      }
    };
    void follow();
    return () => {
      controller.abort();
      cache.stop();
    };
  }, [queryClient]);
}
