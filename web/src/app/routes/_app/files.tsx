import { createFileRoute } from "@tanstack/react-router";

import { FilesPage, filesSearchSchema } from "@/pages/files";

export const Route = createFileRoute("/_app/files")({
  validateSearch: filesSearchSchema,
  component: FilesRoute,
});

function FilesRoute() {
  const search = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <FilesPage
      search={search}
      onSearchChange={(next) => void navigate({ search: next, replace: true })}
    />
  );
}
