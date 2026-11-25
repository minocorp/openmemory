import { createFileRoute } from "@tanstack/react-router";

import { UploadPage } from "@/pages/upload";
import { pageTitle } from "@/shared/lib/page-title";

export const Route = createFileRoute("/upload/$token")({
  head: () => pageTitle("Upload"),
  component: UploadRoute,
});

function UploadRoute() {
  const { token } = Route.useParams();
  return <UploadPage token={token} />;
}
