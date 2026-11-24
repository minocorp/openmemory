import { useNavigate } from "@tanstack/react-router";

import { SignUpFlow } from "@/features/passkey-auth";

type RegisterPageProps = { token: string | undefined };

export function RegisterPage({ token }: RegisterPageProps) {
  const navigate = useNavigate();
  if (!token) {
    return (
      <main className="flex min-h-svh items-center justify-center p-6">
        <div className="max-w-sm">
          <h1 className="text-xl font-semibold tracking-tight">This invite link is incomplete</h1>
          <p className="mt-2 text-sm text-muted-foreground">Ask for a new invite link.</p>
        </div>
      </main>
    );
  }
  return <SignUpFlow kind="invite" token={token} onDone={() => void navigate({ to: "/" })} />;
}
