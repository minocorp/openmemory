import { createFileRoute } from "@tanstack/react-router";

import { pageSearchSchema, PageSkeleton, pathTargetQuery, PathScreen } from "@/pages/page";
import { baseName, normalizePath } from "@/shared/lib/page-path";
import { pageTitle } from "@/shared/lib/page-title";

const pathOf = (params: { _splat?: string | undefined }) => normalizePath(params["_splat"] ?? "");

export const Route = createFileRoute("/_app/p/$")({
  head: ({ params }) => pageTitle(baseName(pathOf(params))),
  validateSearch: pageSearchSchema,
  loader: ({ context, params }) =>
    context.queryClient.ensureQueryData(pathTargetQuery(pathOf(params))),
  pendingComponent: PageSkeleton,
  component: PageRoute,
});

function PageRoute() {
  const params = Route.useParams();
  const search = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <PathScreen
      path={pathOf(params)}
      search={search}
      onSearchChange={(next) => void navigate({ search: next })}
    />
  );
}
