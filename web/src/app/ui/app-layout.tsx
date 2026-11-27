import { Outlet } from "@tanstack/react-router";

import { CommandPalette } from "@/widgets/command-palette";
import { MobileTabs } from "@/widgets/mobile-tabs";
import { SidebarTree } from "@/widgets/sidebar-tree";

import { useLiveChanges } from "../model/use-live-changes";

export function AppLayout() {
  useLiveChanges();
  return (
    <div className="flex h-svh flex-col sm:flex-row">
      <SidebarTree />
      <main className="min-h-0 min-w-0 flex-1">
        <Outlet />
      </main>
      <MobileTabs />
      <CommandPalette />
    </div>
  );
}
