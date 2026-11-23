import { useSuspenseQuery } from "@tanstack/react-query";
import { LogOut } from "lucide-react";

import { meQuery } from "@/entities/user";
import { AuthorBadge } from "@/entities/version";
import { useSignOut } from "@/features/passkey-auth";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";

export function UserMenu() {
  const { data: me } = useSuspenseQuery(meQuery());
  const signOut = useSignOut();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        className="flex w-full items-center rounded-md px-2 py-1.5 text-left text-[13.5px] outline-none hover:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-sidebar-ring"
        aria-label="Account menu"
      >
        <AuthorBadge author={me.username} displayName={me.display_name} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" side="top">
        <DropdownMenuItem onClick={() => signOut.mutate()}>
          <LogOut /> Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
