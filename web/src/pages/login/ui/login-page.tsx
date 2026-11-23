import { Link, useRouter } from "@tanstack/react-router";

import { LoginForm } from "@/features/passkey-auth";

import { safeNextPath } from "../model/search";

type LoginPageProps = { next: string | undefined };

export function LoginPage({ next }: LoginPageProps) {
  const router = useRouter();
  return (
    <LoginForm
      onDone={() => router.history.push(safeNextPath(next))}
      recoveryLink={
        <Link to="/recovery" className="underline underline-offset-4">
          Lost your passkey?
        </Link>
      }
    />
  );
}
