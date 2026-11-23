import { useQuery } from "@tanstack/react-query";

import { uploadLinkQuery } from "@/entities/upload-link";
import { describeError, isApiError } from "@/shared/api";
import { Skeleton } from "@/shared/ui/skeleton";

import { UploadCard } from "./upload-card";
import { UploadForm } from "./upload-form";

type UploadPageProps = { token: string };

export function UploadPage({ token }: UploadPageProps) {
  const link = useQuery(uploadLinkQuery(token));

  if (link.isPending) {
    return (
      <UploadCard title="Upload a file">
        <Skeleton className="h-32" />
      </UploadCard>
    );
  }
  if (link.isError) {
    return isApiError(link.error, 404) ? (
      <UploadCard title="This link has expired or was already used" />
    ) : (
      <UploadCard title="Could not load the link">
        <p className="text-sm text-muted-foreground">{describeError(link.error)}</p>
      </UploadCard>
    );
  }
  return <UploadForm token={token} path={link.data.path} />;
}
