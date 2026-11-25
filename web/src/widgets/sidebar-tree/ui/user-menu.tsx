import { useSuspenseQuery } from "@tanstack/react-query";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { ChevronsUpDown, LogOut, Settings } from "lucide-react";

import { meQuery } from "@/entities/user";
import { AuthorBadge } from "@/entities/version";
import { useSignOut } from "@/features/passkey-auth";
import { cn } from "@/shared/lib/cn";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";

import { rowStyle } from "../config/row-style";

export function UserMenu() {
  const { data: me } = useSuspenseQuery(meQuery());
  const signOut = useSignOut();
  const navigate = useNavigate();
  const isInSettings = useRouterState({
    select: (state) => state.location.pathname.startsWith("/settings"),
  });

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        className={cn(rowStyle, "h-8 px-2 aria-expanded:bg-sidebar-accent")}
        aria-label="Account menu"
        aria-current={isInSettings ? "page" : undefined}
      >
        <AuthorBadge
          author={me.username}
          displayName={me.display_name}
          className="flex-1 font-medium"
        />
        <ChevronsUpDown className="size-3.5 shrink-0 text-muted-foreground" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" side="top" className="w-(--anchor-width)">
        <DropdownMenuItem onClick={() => void navigate({ to: "/settings" })}>
          <Settings /> Settings
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem onClick={() => signOut.mutate()}>
          <LogOut /> Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
