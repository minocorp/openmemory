export function MissingToken() {
  return (
    <main className="flex min-h-svh items-center justify-center p-6">
      <div className="max-w-sm">
        <h1 className="text-xl font-semibold">This link is incomplete</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Open the full link the server printed on first start.
        </p>
      </div>
    </main>
  );
}
