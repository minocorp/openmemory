import type { ReactNode } from "react";

type AuthCardProps = {
  title: string;
  description?: string;
  children: ReactNode;
};

export function AuthCard({ title, description, children }: AuthCardProps) {
  return (
    <main className="flex min-h-svh items-center justify-center bg-background p-6">
      <div className="w-full max-w-sm">
        <p className="mb-8 text-sm font-semibold tracking-tight">Memory</p>
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        {description ? <p className="mt-2 text-sm text-muted-foreground">{description}</p> : null}
        <div className="mt-8">{children}</div>
      </div>
    </main>
  );
}
