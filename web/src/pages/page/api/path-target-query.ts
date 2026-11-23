import { queryOptions } from "@tanstack/react-query";

import { getFileByPath } from "@/entities/file";
import { getPageByPath, pageQuery } from "@/entities/page";
import { isApiError } from "@/shared/api";

export type PathTarget =
  | { kind: "page"; id: number }
  | { kind: "file"; id: number }
  | { kind: "missing" };

async function resolve(
  path: string,
  seedPage: (page: Awaited<ReturnType<typeof getPageByPath>>) => void,
) {
  try {
    const page = await getPageByPath(path);
    seedPage(page);
    return { kind: "page", id: page.id } satisfies PathTarget;
  } catch (error) {
    if (!isApiError(error, 404)) {
      throw error;
    }
  }
  try {
    const file = await getFileByPath(path);
    return { kind: "file", id: file.id } satisfies PathTarget;
  } catch (error) {
    if (!isApiError(error, 404)) {
      throw error;
    }
  }
  return { kind: "missing" } satisfies PathTarget;
}

export const pathTargetQuery = (path: string) =>
  queryOptions({
    queryKey: ["target", path],
    queryFn: ({ client }): Promise<PathTarget> =>
      resolve(path, (page) => client.setQueryData(pageQuery(page.id).queryKey, page)),
  });
