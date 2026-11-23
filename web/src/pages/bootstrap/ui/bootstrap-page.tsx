import { useNavigate } from "@tanstack/react-router";

import { SignUpFlow } from "@/features/passkey-auth";

import { MissingToken } from "./missing-token";

type BootstrapPageProps = { token: string | undefined };

export function BootstrapPage({ token }: BootstrapPageProps) {
  const navigate = useNavigate();
  if (!token) {
    return <MissingToken />;
  }
  return <SignUpFlow kind="bootstrap" token={token} onDone={() => void navigate({ to: "/" })} />;
}
