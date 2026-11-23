import { describeError } from "@/shared/api";
import { Button } from "@/shared/ui/button";

type ErrorStateProps = {
  error: unknown;
  onRetry: () => void;
};

export function ErrorState({ error, onRetry }: ErrorStateProps) {
  return (
    <div role="alert" className="flex h-full min-h-60 items-center justify-center p-8">
      <div className="max-w-sm text-center">
        <h1 className="text-lg font-semibold">Something went wrong</h1>
        <p className="mt-2 text-sm text-muted-foreground">{describeError(error)}</p>
        <Button variant="outline" className="mt-5" onClick={onRetry}>
          Try again
        </Button>
      </div>
    </div>
  );
}
