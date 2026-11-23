import { createFileRoute } from "@tanstack/react-router";

import { DevicePage, deviceSearchSchema } from "@/pages/approve";

export const Route = createFileRoute("/_auth/device")({
  validateSearch: deviceSearchSchema,
  component: DeviceRoute,
});

function DeviceRoute() {
  const { user_code: userCode } = Route.useSearch();
  const navigate = Route.useNavigate();
  return (
    <DevicePage
      userCode={userCode}
      onCodeEntered={(code) => void navigate({ search: { user_code: code } })}
    />
  );
}
