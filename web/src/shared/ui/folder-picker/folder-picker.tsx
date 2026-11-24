import { useId } from "react";

import { Checkbox } from "@/shared/ui/checkbox";

export const allFolders = "*";

type FolderPickerProps = {
  folders: string[];
  value: string[];
  onChange: (next: string[]) => void;
};

const isAll = (value: string[]) => value.includes(allFolders);

const depthOf = (folder: string) => folder.split("/").length - 1;

const nameOf = (folder: string) => folder.split("/").at(-1) ?? folder;

export function FolderPicker({ folders, value, onChange }: FolderPickerProps) {
  const groupName = useId();
  const all = isAll(value);

  const toggle = (folder: string, isChecked: boolean) =>
    onChange(isChecked ? [...value, folder] : value.filter((entry) => entry !== folder));

  return (
    <fieldset className="grid gap-2 text-sm">
      <legend className="sr-only">Folders</legend>
      <label className="flex items-center gap-2">
        <input
          type="radio"
          name={groupName}
          checked={all}
          onChange={() => onChange([allFolders])}
          className="size-3.5 accent-primary"
        />
        All folders
      </label>
      <label className="flex items-center gap-2">
        <input
          type="radio"
          name={groupName}
          checked={!all}
          onChange={() => onChange(folders.length > 0 ? [] : [allFolders])}
          disabled={folders.length === 0}
          className="size-3.5 accent-primary"
        />
        Only these folders
      </label>
      {all ? null : (
        <ul className="max-h-48 overflow-y-auto rounded-lg border bg-background py-1">
          {folders.map((folder) => (
            <li key={folder} style={{ paddingLeft: `${0.5 + depthOf(folder) * 1}rem` }}>
              <label className="flex items-center gap-2 py-1 pr-2">
                <Checkbox
                  checked={value.includes(folder)}
                  onCheckedChange={(isChecked) => toggle(folder, isChecked)}
                  aria-label={folder}
                />
                {nameOf(folder)}
              </label>
            </li>
          ))}
        </ul>
      )}
    </fieldset>
  );
}
