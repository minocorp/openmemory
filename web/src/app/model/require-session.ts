import type { QueryClient } from "@tanstack/react-query";
import { redirect } from "@tanstack/react-router";

import { meQuery } from "@/entities/user";
import { isApiError } from "@/shared/api";

type Location = { pathname: string; href: string };

export async function requireSession(queryClient: QueryClient, location: Location) {
  try {
    await queryClient.ensureQueryData(meQuery());
  } catch (error) {
    if (isApiError(error, 401)) {
      throw redirect({
        to: "/login",
        search: location.pathname === "/" ? {} : { next: location.href },
      });
    }
    throw error;
  }
}
