import { useNavigate } from "@tanstack/react-router";

import { RecoveryFlow } from "@/features/passkey-auth";

export function RecoveryPage() {
  const navigate = useNavigate();
  return <RecoveryFlow onDone={() => void navigate({ to: "/" })} />;
}
