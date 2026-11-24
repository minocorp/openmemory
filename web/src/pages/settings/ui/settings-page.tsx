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
        className="flex shrink-0 gap-px overflow-x-auto border-b px-3 py-2 sm:w-52 sm:flex-col sm:border-r sm:border-b-0 sm:px-2 sm:py-0"
      >
        <h1 className="hidden h-12 items-center px-2 text-sm font-medium sm:flex">Settings</h1>
        {visibleTabs(me.is_admin).map((entry) => (
          <Link
            key={entry}
            to="/settings"
            search={{ tab: entry }}
            aria-current={entry === current ? "page" : undefined}
            className="flex h-7 items-center rounded-md px-2 text-sm whitespace-nowrap text-muted-foreground -outline-offset-2 transition-colors hover:bg-accent hover:text-foreground aria-[current=page]:bg-accent aria-[current=page]:font-medium aria-[current=page]:text-foreground"
          >
            {tabLabels[entry]}
          </Link>
        ))}
      </nav>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto grid max-w-2xl gap-12 px-4 py-8 sm:px-8 sm:py-12">
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
