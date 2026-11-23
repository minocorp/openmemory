import { queryOptions } from "@tanstack/react-query";

import { client, unwrap } from "@/shared/api";

import type { SearchInput } from "../model/parse-search-input";
import { toSearchResults } from "../model/results";

const resultLimit = 30;

export const searchQuery = ({ text, conditions }: SearchInput) =>
  queryOptions({
    queryKey: ["search", text, conditions],
    queryFn: async () => {
      const view = await unwrap(
        client.GET("/api/search", {
          params: {
            query: {
              ...(text === "" ? {} : { q: text }),
              ...(conditions.length === 0 ? {} : { where: conditions }),
              limit: resultLimit,
            },
          },
        }),
      );
      return { results: toSearchResults(view), noHits: view.no_hits ?? null };
    },
    staleTime: 0,
  });
