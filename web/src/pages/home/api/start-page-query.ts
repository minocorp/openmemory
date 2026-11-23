import { queryOptions } from "@tanstack/react-query";

import { getPageByPath } from "@/entities/page";
import { isApiError } from "@/shared/api";

const startPath = "start";

export const startPageQuery = () =>
  queryOptions({
    queryKey: ["target", startPath],
    queryFn: async () => {
      try {
        const page = await getPageByPath(startPath);
        return { exists: true, id: page.id };
      } catch (error) {
        if (isApiError(error, 404)) {
          return { exists: false, id: null };
        }
        throw error;
      }
    },
  });
