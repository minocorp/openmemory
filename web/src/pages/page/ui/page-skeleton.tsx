import { Skeleton } from "@/shared/ui/skeleton";

export function PageSkeleton() {
  return (
    <div className="mx-auto w-full max-w-[688px] px-6 pt-16">
      <Skeleton className="h-9 w-64" />
      <div className="mt-8 grid gap-3">
        <Skeleton className="h-4 w-full" />
        <Skeleton className="h-4 w-11/12" />
        <Skeleton className="h-4 w-2/3" />
      </div>
    </div>
  );
}
