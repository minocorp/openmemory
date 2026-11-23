import { z } from "@/shared/lib/zod";

export const settingsTabs = ["account", "people", "connections", "system"] as const;

export type SettingsTab = (typeof settingsTabs)[number];

export const settingsSearchSchema = z.object({
  tab: z.enum(settingsTabs).default("account"),
});

export type SettingsSearch = z.infer<typeof settingsSearchSchema>;

const adminTabs: SettingsTab[] = ["people", "system"];

export function visibleTabs(isAdmin: boolean): SettingsTab[] {
  return settingsTabs.filter((tab) => isAdmin || !adminTabs.includes(tab));
}

export function resolveTab(tab: SettingsTab, isAdmin: boolean): SettingsTab {
  return visibleTabs(isAdmin).includes(tab) ? tab : "account";
}
