import { lazy, Suspense, type ComponentProps } from "react";

import { Skeleton } from "@/shared/ui/skeleton";

const FileViewer = lazy(async () => ({
  default: (await import("@/widgets/file-viewer")).FileViewer,
}));

export function LazyFileViewer(props: ComponentProps<typeof FileViewer>) {
  return (
    <Suspense fallback={<Skeleton className="m-4 h-64" />}>
      <FileViewer {...props} />
    </Suspense>
  );
}
