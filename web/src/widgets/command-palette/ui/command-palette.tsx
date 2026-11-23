import { useEffect } from "react";

import { setSearchOpen, useIsSearchOpen } from "@/shared/lib/search-palette";
import { CommandDialog } from "@/shared/ui/command";

import { PaletteBody } from "./palette-body";

export function CommandPalette() {
  const isOpen = useIsSearchOpen();

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSearchOpen(!isOpen);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isOpen]);

  return (
    <CommandDialog open={isOpen} onOpenChange={setSearchOpen} label="Search">
      <PaletteBody onClose={() => setSearchOpen(false)} />
    </CommandDialog>
  );
}
