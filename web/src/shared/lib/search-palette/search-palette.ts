import { useSyncExternalStore } from "react";

const listeners = new Set<() => void>();
let isOpen = false;

function setOpen(next: boolean) {
  isOpen = next;
  listeners.forEach((listener) => listener());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export const openSearch = () => setOpen(true);
export const setSearchOpen = setOpen;

export function useIsSearchOpen(): boolean {
  return useSyncExternalStore(subscribe, () => isOpen);
}
