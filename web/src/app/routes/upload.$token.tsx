import { createFileRoute } from "@tanstack/react-router";

import { UploadPage } from "@/pages/upload";

export const Route = createFileRoute("/upload/$token")({ component: UploadRoute });

function UploadRoute() {
  const { token } = Route.useParams();
  return <UploadPage token={token} />;
}
