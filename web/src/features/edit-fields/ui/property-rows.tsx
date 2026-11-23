import type { Page } from "@/entities/page";

import { useSaveField } from "../api/use-save-field";
import { AddPropertyRow } from "./add-property-row";
import { PropertyRow } from "./property-row";

type PropertyRowsProps = {
  page: Page;
  changedLines: ReadonlySet<string>;
};

export function PropertyRows({ page, changedLines }: PropertyRowsProps) {
  const save = useSaveField();
  const handleSave = (key: string, values: string[]) => save.mutate({ page, key, values });
  const isChanged = (key: string) => [...changedLines].some((line) => line.startsWith(`${key}:`));

  return (
    <div className="mb-6 border-b pb-5">
      {page.fields.length > 0 ? (
        <dl className="mb-1 grid grid-cols-[minmax(6rem,max-content)_1fr] gap-x-4 gap-y-1">
          {page.fields.map((field) => (
            <PropertyRow
              key={field.key}
              field={field}
              isChanged={isChanged(field.key)}
              isSaving={save.isPending}
              onSave={handleSave}
            />
          ))}
        </dl>
      ) : null}
      <AddPropertyRow
        existingKeys={page.fields.map((field) => field.key)}
        isSaving={save.isPending}
        onAdd={handleSave}
      />
    </div>
  );
}
