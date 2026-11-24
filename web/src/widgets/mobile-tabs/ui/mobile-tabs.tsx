import { Link, useRouterState } from "@tanstack/react-router";
import { FileText, Folder, Home, Settings } from "lucide-react";

import { cn } from "@/shared/lib/cn";

const tabs = [
  { to: "/", label: "Home", icon: Home },
  { to: "/pages", label: "Pages", icon: FileText },
  { to: "/files", label: "Files", icon: Folder },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

export function MobileTabs() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  return (
    <nav
      aria-label="Sections"
      className="flex shrink-0 border-t bg-background pb-[env(safe-area-inset-bottom)] sm:hidden"
    >
      {tabs.map(({ to, label, icon: Icon }) => (
        <Link
          key={to}
          to={to}
          activeOptions={{ exact: to === "/" }}
          className={cn(
            "flex flex-1 flex-col items-center gap-0.5 py-2 text-xs text-muted-foreground",
            isTabActive(to, pathname) && "font-medium text-foreground",
          )}
        >
          <Icon className="size-5" />
          {label}
        </Link>
      ))}
    </nav>
  );
}

function isTabActive(to: string, pathname: string): boolean {
  if (to === "/pages") {
    return pathname === "/pages" || pathname.startsWith("/p/");
  }
  return to === "/" ? pathname === "/" : pathname.startsWith(to);
}
