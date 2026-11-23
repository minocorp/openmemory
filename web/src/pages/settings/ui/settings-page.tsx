import { useSuspenseQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { LogOut } from "lucide-react";
import { Suspense } from "react";

import { type Me, meQuery, PersonName } from "@/entities/user";
import { ProfileSection } from "@/features/edit-profile";
import { ConnectionsSection } from "@/features/manage-connections";
import { PasskeysSection } from "@/features/manage-passkeys";
import { InvitesSection, UsersSection } from "@/features/manage-people";
import { useSignOut } from "@/features/passkey-auth";
import { BackupSection, TransferSection } from "@/features/system-admin";
import { Button } from "@/shared/ui/button";
import { Skeleton } from "@/shared/ui/skeleton";

import { resolveTab, type SettingsTab, visibleTabs } from "../model/search";

const tabLabels: Record<SettingsTab, string> = {
  account: "Account",
  people: "People",
  connections: "Connections",
  system: "System",
};

type SettingsPageProps = { tab: SettingsTab };

export function SettingsPage({ tab }: SettingsPageProps) {
  const { data: me } = useSuspenseQuery(meQuery());
  const current = resolveTab(tab, me.is_admin);

  return (
    <div className="flex h-full flex-col sm:flex-row">
      <nav
        aria-label="Settings"
        className="flex shrink-0 gap-1 overflow-x-auto border-b px-4 py-2 sm:w-48 sm:flex-col sm:border-r sm:border-b-0 sm:px-3 sm:pt-5"
      >
        <h1 className="hidden px-2 pb-2 text-sm font-semibold sm:block">Settings</h1>
        {visibleTabs(me.is_admin).map((entry) => (
          <Link
            key={entry}
            to="/settings"
            search={{ tab: entry }}
            aria-current={entry === current ? "page" : undefined}
            className="rounded-md px-2 py-1 text-[13.5px] whitespace-nowrap hover:bg-accent aria-[current=page]:bg-accent aria-[current=page]:font-semibold"
          >
            {tabLabels[entry]}
          </Link>
        ))}
      </nav>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto grid max-w-2xl gap-8 px-4 py-6 sm:px-8 sm:py-8">
          <Suspense fallback={<Skeleton className="h-40" />}>
            <TabContent tab={current} me={me} />
          </Suspense>
        </div>
      </div>
    </div>
  );
}

function TabContent({ tab, me }: { tab: SettingsTab; me: Me }) {
  if (tab === "people") {
    return (
      <>
        <InvitesSection />
        <UsersSection />
      </>
    );
  }
  if (tab === "connections") {
    return <ConnectionsSection />;
  }
  if (tab === "system") {
    return (
      <>
        <BackupSection />
        <TransferSection />
      </>
    );
  }
  return <AccountTab me={me} />;
}

function AccountTab({ me }: { me: Me }) {
  const signOut = useSignOut();
  return (
    <>
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm">
          Signed in as <PersonName displayName={me.display_name} username={me.username} />
        </p>
        <Button
          variant="outline"
          size="sm"
          disabled={signOut.isPending}
          onClick={() => signOut.mutate()}
        >
          <LogOut /> Sign out
        </Button>
      </div>
      <ProfileSection />
      <PasskeysSection />
    </>
  );
}
