import { ConsentNotice } from "./consent-notice";
import { ConsentScreen } from "./consent-screen";

type ApprovePageProps = { consent: string | undefined };

export function ApprovePage({ consent }: ApprovePageProps) {
  if (!consent) {
    return <ConsentNotice title="This link is incomplete" />;
  }
  return <ConsentScreen key={consent} reference={consent} />;
}
