import type { ReactNode } from "react";

import { Brand } from "@/shared/ui/brand";

type UploadCardProps = { title: string; children?: ReactNode };

export function UploadCard({ title, children }: UploadCardProps) {
  return (
    <main className="flex min-h-svh justify-center bg-background p-6 max-sm:pt-10 sm:items-center">
      <div className="w-full max-w-md">
        <Brand className="mb-8" />
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        <div className="mt-6">{children}</div>
      </div>
    </main>
  );
}
