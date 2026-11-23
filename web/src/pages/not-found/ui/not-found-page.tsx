import { Link } from "@tanstack/react-router";

export function NotFoundPage() {
  return (
    <main className="flex min-h-svh flex-col items-center justify-center gap-2">
      <h1 className="text-2xl font-medium">Page not found</h1>
      <Link to="/" className="text-muted-foreground underline underline-offset-4">
        Back to memory
      </Link>
    </main>
  );
}
