import type { ReactNode } from "react";

type ConsentCardProps = { title: string; children: ReactNode };

export function ConsentCard({ title, children }: ConsentCardProps) {
  return (
    <main className="flex min-h-svh justify-center bg-background p-6 max-sm:pt-10 sm:items-center">
      <div className="w-full max-w-md">
        <p className="mb-8 text-sm font-semibold tracking-tight">Memory</p>
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        <div className="mt-6">{children}</div>
      </div>
    </main>
  );
}
